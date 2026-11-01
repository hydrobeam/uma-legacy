// The career page's run machine (src/career/run.ts `createRun`) on the real answer store (career/state.ts) with a fake
// engine (each job a promise the test settles; a restart rejects a job still waiting on the engine's start and leaves
// one in flight unanswered, as engine.ts does), a fake clock and timers, and fakes of the page's parts. Staleness: late
// replies of older runs are dropped, a check's own writes keep it running while a new plan stops it, the timers end
// with the run, the comparison goes stale with every new run, and the last visit's answer is drawn after a reload.
// node --test 'test/unit/*.test.*' (in webapp)
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { PAUSE_MS, createRun } from '../../src/career/run.ts';
import { jobKey, planKey } from '../../src/career/keys.js';
import * as store from '../../src/career/state.ts';
import { fakeClock, flush } from './fakes.mjs';

const { answer, answerCareer, runView } = store;

// ---------- the fakes
// the engine: each job held for the test. `cold`: jobs sent now wait on the engine's start (a restart rejects them);
// otherwise they are in flight (a restart leaves them unanswered)
function fakeEngine() {
  const engine = { jobs: [], restarts: 0, saves: 0, cold: false, fingerprint: 'wasm1|engine1', kill: null };
  const killed = () => {
    engine.kill = engine.kill || Promise.withResolvers();
    return engine.kill.promise;
  };
  engine.run = (job, progress, background = false) =>
    new Promise((resolve, reject) =>
      engine.jobs.push({ job, progress, background, resolve, reject, waiting: engine.cold, live: true }),
    );
  engine.busy = () => engine.jobs.some((j) => j.live && !j.background);
  engine.restart = () => {
    engine.restarts++;
    for (const j of engine.jobs.filter((x) => x.live)) {
      j.live = false;
      if (j.waiting) j.reject(new Error('superseded'));
    }
    engine.kill?.resolve();
    engine.kill = null;
  };
  engine.nextKill = killed;
  engine.saveState = () => engine.saves++;
  engine.key = () => engine.fingerprint;
  // answer job n (from 0) with the report r
  engine.answer = (n, r, ms = 1500) => {
    const j = engine.jobs[n];
    if (!j.live) return; // a killed job never answers
    j.live = false;
    j.resolve({ r, ms });
  };
  return engine;
}

// the reports: a search's best careers, a plan, a check step
const career = (name) => ({ trainee: { id: 1001, name }, parents: [0, 1], career: name, p_all: 0.01 });
const searched = (...names) => ({ results: names.map(career), candidates: 120 });
const planned = (pick) => ({
  next: { choice: career(pick), options: [career(pick)], v0: 9, outcomes: { career: pick } },
});
const check = (pending, pairs = null, plan = 'pick') => ({ next: { robust: { plan, pending, stable: true, pairs } } });

let pages = 0;
// a page: its own job (a target no other page's has, so no answer kept by an earlier test is taken), the machine on
// the fakes. `trainee`: chosen by hand (no plan phase)
function page({ trainee = null } = {}) {
  const clock = fakeClock(),
    engine = fakeEngine(),
    target = ++pages,
    seen = { paints: 0, draws: 0, checkDraws: 0, stops: 0, outdates: 0, shown: [], cleared: 0, saved: [] };
  const job = {
    cmd: 'career',
    targets: [{ group: target }],
    legacies: [],
    trainees: trainee ? [trainee] : [],
    fix: [],
  };
  const deps = {
    engine,
    clock,
    store,
    page: {
      job: () => {
        if (deps.blocked) throw Object.assign(new Error('No target skills.'), { kind: 'empty' });
        return job;
      },
      places: () => (trainee ? `{"trainee":${trainee}}` : '{"trainee":null}'),
      trainee: () => trainee,
      refBase: () => 'base',
      baseKey: () => 'basekey',
      searched: (m, found) => (m.r.results.length ? { kind: 'done', text: found } : { kind: 'error', text: 'none' }),
      loadAnswer: async () => null,
      saveAnswer: (key, value) => seen.saved.push({ key, value }),
    },
    analysis: {
      outdate: () => seen.outdates++,
      stop: () => seen.stops++,
      show: (r) => seen.shown.push(r),
      clear: () => seen.cleared++,
      keepBaseline: () => undefined,
    },
    view: {
      shown: () => answerCareer.peek(),
      paint: () => seen.paints++,
      drawAll: () => seen.draws++,
      drawCheck: () => seen.checkDraws++,
    },
    blocked: false,
  };
  return { run: createRun(deps), deps, clock, engine, seen, job };
}
// an edit and its pause: the search's job is sent
async function edited(p) {
  p.run.schedule();
  p.clock.advance(PAUSE_MS);
  await flush();
}
// the search answered with `names`, then the plan with `pick`
async function answered(p, n, names, pick) {
  p.engine.answer(n, searched(...names));
  await flush();
  if (pick !== undefined) {
    p.engine.answer(n + 1, planned(pick));
    await flush();
  }
}
const sent = (p) => p.engine.jobs.length;

// ---------- staleness
test("an edit during the search: the old run's late answer is dropped, the new run's is shown", async () => {
  const p = page();
  await edited(p);
  assert.equal(sent(p), 1);
  p.run.schedule(); // an edit: the job is the same, so the run under way goes on
  assert.equal(p.engine.restarts, 0, 'an edit that leaves the job as it is kills nothing');
  p.job.targets.push({ group: 900 }); // a real edit
  p.run.schedule();
  assert.equal(p.engine.restarts, 1, 'a job in flight is cut short');
  const shownBefore = answer.peek();
  p.engine.jobs[0].live = true; // (as if the worker answered before the kill landed)
  p.engine.answer(0, searched('old'));
  await flush();
  assert.equal(answer.peek(), shownBefore, 'the old run draws nothing');
  assert.equal(runView.peek().searching, true, 'the new run waits out its pause');
  p.clock.advance(PAUSE_MS);
  await flush();
  await answered(p, 1, ['new'], 'pick');
  assert.equal(answer.peek().last.r.results[0].career, 'new');
  assert.equal(answer.peek().planState.r.choice.career, 'pick');
  assert.equal(runView.peek().searching, false);
});

test('an edit during the plan: its late progress and answer are dropped', async () => {
  const p = page();
  await edited(p);
  await answered(p, 0, ['best']);
  assert.equal(runView.peek().phase, 'plan');
  assert.deepEqual(p.engine.jobs[1].job.next, {}, "the plan job's next");
  assert.equal(p.engine.jobs[1].job.no_reports, true);
  p.job.targets.push({ group: 901 });
  p.run.schedule();
  const before = runView.peek();
  p.engine.jobs[1].progress({ ph: 2, k: 0, n: 3 });
  assert.equal(runView.peek(), before, "the old plan's progress writes nothing");
  const shownBefore = answer.peek();
  p.engine.jobs[1].live = true;
  p.engine.answer(1, planned('stale'));
  await flush();
  assert.equal(answer.peek(), shownBefore, 'nothing of the old run is drawn');
  assert.equal(p.engine.saves, 0, "the old plan's work is not saved");
});

test('a restart rejects a job waiting on the start: no error is shown, and the next edit can stop the next run', async () => {
  const p = page();
  p.engine.cold = true; // the first search waits on the wasm compiling
  await edited(p);
  p.job.targets.push({ group: 902 });
  p.run.schedule(); // kills: the waiting job rejects with 'superseded'
  await flush();
  assert.notEqual(answer.peek().status.kind, 'error', "the superseded job's rejection is not shown");
  assert.equal(runView.peek().searching, true);
  p.engine.cold = false;
  p.clock.advance(PAUSE_MS);
  await flush();
  assert.equal(sent(p), 2);
  assert.equal(p.engine.busy(), true, 'the new run is under way');
  p.job.targets.push({ group: 903 });
  p.run.schedule();
  assert.equal(p.engine.restarts, 2, 'so the next edit stops it');
});

test('a Blocked job ends the run unsearched, with its message; a thrown job error the same', async () => {
  const p = page();
  p.deps.blocked = true;
  await edited(p);
  assert.equal(sent(p), 0);
  assert.equal(answer.peek().status.kind, 'empty');
  assert.equal(answer.peek().status.text, 'No target skills.');
  assert.equal(runView.peek().searching, false);
  assert.equal(p.seen.cleared, 1, 'the analysis is cleared');
  assert.equal(p.clock.pending(), 0);
});

test('the engine failing or refusing: the error is shown and the run ends', async () => {
  const p = page();
  await edited(p);
  p.engine.jobs[0].live = false;
  p.engine.jobs[0].reject(new Error('the solver failed to start'));
  await flush();
  assert.equal(answer.peek().status.text, 'the solver failed to start');
  assert.equal(runView.peek().searching, false);
  p.job.targets.push({ group: 904 });
  await edited(p);
  p.engine.answer(1, { error: 'bad job' });
  await flush();
  assert.equal(answer.peek().status.text, 'Not searched: bad job');
  // the same job again (Search, or an edit and its undoing): a failed answer does not stand, so it runs again
  p.run.schedule();
  assert.equal(runView.peek().searching, true);
  p.clock.advance(PAUSE_MS);
  await flush();
  assert.equal(sent(p), 3);
});

// ---------- timers
test('the status line ticks each second while a job runs, and every timer ends with the run', async () => {
  const p = page();
  p.run.schedule();
  assert.equal(p.clock.pending(), 1, 'the pause');
  p.run.schedule(); // the same job: nothing new
  p.job.targets.push({ group: 905 });
  p.run.schedule(); // a new edit replaces the pause
  assert.equal(p.clock.pending(), 1);
  p.clock.advance(PAUSE_MS);
  await flush();
  assert.equal(p.clock.pending(), 1, 'the ticker');
  const paints = p.seen.paints;
  p.clock.advance(3000);
  assert.equal(p.seen.paints - paints, 3, 'a repaint each second');
  await answered(p, 0, ['best']);
  p.clock.advance(1000);
  assert.equal(p.clock.pending(), 1, 'one ticker through the plan phase');
  p.engine.answer(1, planned('pick'));
  await flush();
  assert.equal(p.clock.pending(), 0, 'no ticker after the answer');
  const done = p.seen.paints;
  p.clock.advance(5000);
  assert.equal(p.seen.paints, done, 'no repaint after');
  // an edit during a search, then the search: one ticker at a time
  p.job.targets.push({ group: 906 });
  await edited(p);
  p.job.targets.push({ group: 907 });
  await edited(p);
  assert.equal(p.clock.pending(), 1, 'the killed run left no ticker');
});

// ---------- the comparison follows the runs
test('every new run makes the comparison stale before anything awaits (analysis.js keeps no run id of its own)', async () => {
  const p = page();
  p.run.schedule();
  assert.deepEqual([p.seen.stops, p.seen.outdates], [1, 0], 'an edit stops it');
  p.clock.advance(PAUSE_MS);
  assert.deepEqual([p.seen.stops, p.seen.outdates], [1, 1], 'the search outdates it, synchronously');
  p.run.search(); // the Search path: a run without an edit
  assert.equal(p.seen.outdates, 2);
  p.run.schedule(); // the same job: no new run, nothing goes stale
  assert.deepEqual([p.seen.stops, p.seen.outdates], [1, 2]);
});

// ---------- the check through the machine
test("a run's check sends every variant through its own writes, and a new plan stops it", async () => {
  const p = page();
  await edited(p);
  await answered(p, 0, ['best'], 'pick');
  const plan = answer.peek().planState;
  assert.equal(plan.state, 'done');
  assert.equal(p.engine.saves, 1, "a plan made: the worker's work is kept");
  assert.equal(sent(p), 3, 'the first check job');
  assert.deepEqual(p.engine.jobs[2].job.next, { robust: 'next', pair_search: true });
  assert.equal(p.engine.jobs[2].background, true);
  for (const [n, reply] of [check(2), check(1), check(0)].entries()) {
    p.engine.answer(2 + n, reply);
    await flush();
  }
  assert.equal(answer.peek().planState.id, plan.id, 'the same plan, its record replaced by each step');
  assert.equal(sent(p), 6, 'the pair search after the last variant');
  // an edit, and its run ends on a new plan: the check's late reply writes nothing
  p.job.targets.push({ group: 908 });
  await edited(p);
  await answered(p, 6, ['best'], 'pick');
  const fresh = answer.peek().planState;
  p.engine.jobs[5].live = true;
  p.engine.answer(5, check(0, { trainees: [] }));
  await flush();
  assert.equal(answer.peek().planState.r.robust, undefined, "the old plan's check did not write the new one");
  assert.equal(answer.peek().planState.id, fresh.id);
});

test("the engine restarted under a check: the check's race ends with the kill, and a new run's edit leaves it unmarked", async () => {
  // no edit: the kill loses the check, which is marked unavailable and drawn
  const alone = page();
  await edited(alone);
  await answered(alone, 0, ['best'], 'pick');
  alone.engine.restart();
  await flush();
  assert.equal(answer.peek().planState.r.check_unavailable, true);
  assert.equal(alone.seen.checkDraws, 1);
  // an edit first: the check is stale and marks nothing
  const p = page();
  await edited(p);
  await answered(p, 0, ['best'], 'pick');
  assert.equal(sent(p), 3);
  p.job.targets.push({ group: 909 });
  p.run.schedule(); // the check's job is background: the engine is not busy, so nothing is killed
  assert.equal(p.engine.restarts, 0);
  p.engine.restart(); // (a stopped comparison kills the worker)
  await flush();
  assert.notEqual(answer.peek().planState.r?.check_unavailable, true, 'a stale check marks nothing');
});

// ---------- the answers kept and the reload
test('an edit and its undoing: the kept answer is drawn again with no job, and its unfinished check resumes', async () => {
  const p = page();
  await edited(p);
  await answered(p, 0, ['best'], 'pick');
  p.engine.answer(2, check(1));
  await flush();
  // another job, answered (its answer on show), then a third under way (its ticker running)
  p.job.targets.push({ group: 910 });
  await edited(p);
  await answered(p, sent(p) - 1, ['other'], 'pick');
  p.job.targets.push({ group: 911 });
  await edited(p);
  const jobs = sent(p);
  // back to the first job: drawn from the answers kept
  p.job.targets.splice(1);
  p.run.schedule();
  p.clock.advance(PAUSE_MS);
  await flush();
  assert.equal(answer.peek().last.r.results[0].career, 'best');
  assert.equal(answer.peek().planState.r.robust.pending, 1, 'the kept answer, with its check as far as it ran');
  assert.equal(runView.peek().searching, false);
  assert.equal(sent(p), jobs + 1, 'no search: one job, its check resumed');
  assert.equal(p.engine.jobs.at(-1).background, true);
  assert.equal(p.clock.pending(), 0, "the third run's ticker ends with it");
});

test('a reload: the answer kept for this setup and these fingerprints is drawn at once, its check resumes', async () => {
  const first = page();
  await edited(first);
  await answered(first, 0, ['best'], 'pick');
  const [{ key, value }] = first.seen.saved;
  assert.ok(key.startsWith('wasm1|engine1\n'), "the saved answer's key carries the fingerprints");
  // the next visit, the same setup (made here on another page's job: the first page's answer is in the cache too)
  const again = page(),
    savedFor = (p) => {
      const saved = { key: `${p.engine.fingerprint}\n${jobKey(p.job)}\n${p.deps.page.places()}`, value };
      return async () => saved;
    };
  again.deps.page.loadAnswer = savedFor(again);
  await again.run.start();
  await flush();
  assert.equal(again.engine.jobs.filter((j) => !j.background).length, 0, 'nothing searched');
  assert.equal(answer.peek().last.r.results[0].career, 'best');
  assert.notEqual(answer.peek().planState.id, value.planState.id, 'the saved plan gets an id of this page');
  assert.equal(again.engine.jobs.length, 1, 'its check resumes');
  // another solver or data release: the saved answer is not taken
  const other = page();
  other.deps.page.loadAnswer = savedFor(other);
  other.engine.fingerprint = 'wasm2|engine1'; // (after the key was made: the saved one is of the old release)
  await other.run.start();
  await flush();
  assert.equal(other.engine.jobs.length, 1, 'a search is sent');
  assert.equal(other.engine.jobs[0].background, false);
});

test("Compare's kept plan: the plan phase takes it and sends no plan job", async () => {
  const p = page();
  const jp = { ...p.job, next: {}, no_reports: true };
  store.keepComparisonPlan(jp, planned('kept').next, 7);
  await edited(p);
  p.engine.answer(0, searched('best'));
  await flush();
  assert.equal(answer.peek().planState.r.choice.career, 'kept');
  assert.equal(answer.peek().planState.ms, 7);
  assert.equal(p.engine.jobs.filter((j) => !j.background).length, 1, 'only the search was sent');
  // its check runs, and each step reaches the kept plan too: a run that takes it again finds the check as far as it ran
  p.engine.answer(1, check(1, null, 'kept'));
  await flush();
  assert.deepEqual(store.keptPlan(planKey(jp)).r.robust, check(1, null, 'kept').next.robust);
});

test('a trainee chosen by hand: no plan phase, the best odds shown', async () => {
  const p = page({ trainee: 1001 });
  await edited(p);
  p.engine.answer(0, searched('mine'));
  await flush();
  assert.equal(answer.peek().planState.state, 'none');
  assert.equal(sent(p), 1);
  assert.equal(p.seen.shown.at(-1).career, 'mine');
});

// review-7 P1: Compare's kept plan whose check an edit cut short (its record holds the variants run so far): a run that
// takes it resumes the check, as a kept answer's does; one whose check is done, or unavailable, sends none
test("Compare's kept plan with its check cut short: the run that takes it resumes the check", async () => {
  const p = page();
  const jp = { ...p.job, next: {}, no_reports: true },
    plan = planned('kept').next;
  store.keepComparisonPlan(jp, { ...plan, robust: check(1, null, 'kept').next.robust }, 7);
  await edited(p);
  p.engine.answer(0, searched('best'));
  await flush();
  assert.equal(answer.peek().planState.r.robust.pending, 1);
  assert.equal(p.engine.jobs.length, 2, 'a check job');
  assert.equal(p.engine.jobs[1].background, true);
  // its check done, or unavailable: nothing more is sent
  for (const r of [
    { ...plan, robust: check(0, { trainees: [] }, 'kept').next.robust },
    { ...plan, check_unavailable: true },
  ]) {
    const q = page();
    store.keepComparisonPlan({ ...q.job, next: {}, no_reports: true }, r, 7);
    await edited(q);
    q.engine.answer(0, searched('best'));
    await flush();
    assert.equal(q.engine.jobs.length, 1, 'only the search');
  }
});
