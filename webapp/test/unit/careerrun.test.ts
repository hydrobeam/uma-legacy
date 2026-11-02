// The career page's run machine (src/career/run.ts `createRun`) on the real answer store (career/state.ts) with a fake
// engine (each job a promise the test settles; a restart rejects a job still waiting on the engine's start and leaves
// one in flight unanswered, as engine.ts does), a fake clock and timers, and fakes of the page's parts. Staleness: late
// replies of older runs are dropped, a check's own writes keep it running while a new plan stops it, the timers end
// with the run, the comparison goes stale with every new run, and the last visit's answer is drawn after a reload.
// npx vitest run (in webapp)
import assert from 'node:assert/strict';
import { test, vi } from 'vitest';
import { PAUSE_MS, type RunDeps, createRun } from '../../src/career/run.ts';
import { jobKey, planKey } from '../../src/career/keys.ts';
import { stateKey } from '../../src/engine.ts';
import * as store from '../../src/career/state.ts';
import type { AnswerView } from '../../src/career/state.ts';
import * as analysisStore from '../../src/career/analysis/state.ts';
import { Comparison, type ComparisonDeps } from '../../src/career/analysis/compare.ts';
import { Blocked } from '../../src/career/job.ts';
import type { CareerAnswer, CareerJob, CareerReply, CareerReport, PlanCheck } from '../../src/career/job.ts';
import type { PairSearch } from '../../src/gen/solver.ts';
import type { PlanProgress, SolverRefusal } from '../../src/solver.ts';
import type { DeepReadonly } from '../../src/store.ts';
import { must } from '../../src/must.ts';
import { fakeClock, flush } from './fakes.ts';
import { careerJob, careerReport, careerSearch, checkReport, pairSearch, planOption, planReport } from './reports.ts';

vi.mock('#page-data', () => import('./fakepage.ts')); // run.ts reads the solver's fingerprints from it (engine.ts stateKey)

const { answer, answerCareer, runView } = store;

// ---------- the fakes
interface HeldJob {
  job: CareerJob;
  progress: ((p: PlanProgress) => void) | undefined;
  background: boolean;
  resolve: (answer: CareerAnswer) => void;
  reject: (error: Error) => void;
  waiting: boolean;
  live: boolean;
}
// the engine: each job held for the test. `cold`: jobs sent now wait on the engine's start (a restart rejects them);
// otherwise they are in flight (a restart leaves them unanswered)
function fakeEngine() {
  const jobs: HeldJob[] = [];
  let kill: { promise: Promise<void>; resolve: () => void } | null = null;
  const killed = () => {
    if (!kill) {
      let resolve: () => void = () => undefined;
      const promise = new Promise<void>((r) => {
        resolve = () => r();
      });
      kill = { promise, resolve };
    }
    return kill.promise;
  };
  const engine = {
    jobs,
    restarts: 0,
    saves: 0,
    cold: false,
    run: (job: CareerJob, progress?: (p: PlanProgress) => void, background = false) =>
      new Promise<CareerAnswer>((resolve, reject) => {
        jobs.push({ job, progress, background, resolve, reject, waiting: engine.cold, live: true });
      }),
    busy: () => jobs.some((j) => j.live && !j.background),
    restart: () => {
      engine.restarts++;
      for (const j of jobs.filter((x) => x.live)) {
        j.live = false;
        if (j.waiting) j.reject(new Error('superseded'));
      }
      kill?.resolve();
      kill = null;
    },
    nextKill: killed,
    saveState: () => {
      engine.saves++;
    },
    // answer job n (from 0) with the report r
    answer: (n: number, r: CareerReply | SolverRefusal, ms = 1500) => {
      const j = must(jobs[n], `job ${n}`);
      if (!j.live) return; // a killed job never answers
      j.live = false;
      j.resolve({ r, ms });
    },
  };
  return engine;
}

// the reports: a search's best careers, a plan (a plan job's search: no career reports, its plan in `next`), a check
// step (a kept plan's follow-up, `kind` "check": the check alone)
const career = (name: string) =>
  careerReport({
    trainee: { id: 1001, name, outfits: [], covers: [], schedule_outfit: 0 },
    parents: [0, 1],
    career: name,
    p_all: 0.01,
  });
const searched = (...names: string[]) => careerSearch({ results: names.map(career), candidates: 120 });
const plan = (pick: string) =>
  planReport({
    choice: career(pick),
    options: [planOption(career(pick))],
    v0: 9,
    outcomes: { trainee: 1001, parents: [0, 1], career: pick, uncv: [], fixed: [] },
  });
const planned = (pick: string) => careerSearch({ results: [], next: plan(pick) });
const check = (
  pending: number,
  pairs: Partial<PairSearch> | null = null,
  pick = 'pick',
): { kind: 'check'; robust: PlanCheck } => ({
  kind: 'check',
  robust: checkReport({ plan: pick, pending, stable: true, pairs: pairs && pairSearch(pairs) }),
});
// an answer's plan, made (throws: none)
const doneOf = (a: AnswerView) => {
  const ps = a.planState;
  assert.ok(ps.state === 'done', 'a plan');
  return ps;
};

// what the run machine tells the comparison, counted (a comparison is made only as the page's own class, so the counts
// are a subclass's, its machine never run)
interface Seen {
  paints: number;
  stops: number;
  outdates: number;
  shown: (DeepReadonly<CareerReport> | null)[];
  cleared: number;
  saved: { key: string; value: AnswerView }[];
}
const idle: ComparisonDeps = {
  engine: {
    run: () => Promise.reject(new Error('no comparison runs here')),
    restart: () => undefined,
    saveState: () => undefined,
  },
  clock: { setTimer: () => 0 },
  answers: store,
  store: analysisStore,
  page: {
    shown: () => null,
    searching: () => false,
    baseKey: () => '',
    targets: () => [],
    isFull: () => false,
    planTop: 0,
    draftOf: () => null,
    draft: () => null,
  },
};
class CountingComparison extends Comparison {
  readonly seen: Seen;
  constructor(seen: Seen) {
    super(idle);
    this.seen = seen;
  }
  override readonly outdate = () => {
    this.seen.outdates++;
  };
  override readonly stop = () => {
    this.seen.stops++;
  };
  override readonly show = (r: DeepReadonly<CareerReport> | null) => {
    this.seen.shown.push(r);
  };
  override readonly clear = () => {
    this.seen.cleared++;
  };
  override readonly keepBaseline = () => undefined;
}

let pages = 0;
// a page: its own job (a target no other page's has, so no answer kept by an earlier test is taken), the machine on
// the fakes. `trainee`: chosen by hand (no plan phase). `state`: the job can't be built (`blocked`) or throws (`broken`)
function page({ trainee = null }: { trainee?: number | null } = {}) {
  const clock = fakeClock(),
    engine = fakeEngine(),
    target = ++pages,
    seen: Seen = { paints: 0, stops: 0, outdates: 0, shown: [], cleared: 0, saved: [] },
    state = { blocked: false, broken: false };
  const base = careerJob({ legacies: [], trainees: trainee ? [trainee] : [], fix: [] });
  const job: CareerJob = { ...base, skills: { ...base.skills, targets: [{ group: target }] } };
  const deps: RunDeps = {
    engine,
    clock,
    store,
    page: {
      job: () => {
        if (state.blocked) throw new Blocked('empty', 'No target skills.');
        if (state.broken) throw new Error('the job could not be built');
        return job;
      },
      pool: () => ['a', 'b'],
      places: () => (trainee ? `{"trainee":${trainee}}` : '{"trainee":null}'),
      trainee: () => trainee,
      refBase: () => 'base',
      baseKey: () => 'basekey',
      searched: (m, found) => (m.r.results.length ? { kind: 'done', text: found } : { kind: 'error', text: 'none' }),
      loadAnswer: async () => null,
      saveAnswer: (key, value) => {
        seen.saved.push({ key, value });
      },
    },
    analysis: new CountingComparison(seen),
    view: {
      shown: () => answerCareer.peek(),
      paint: () => {
        seen.paints++;
      },
    },
  };
  return { run: createRun(deps), deps, state, clock, engine, seen, job };
}
type Page = ReturnType<typeof page>;
// the job sent `i`th (from the end when negative)
const held = (p: Page, i: number) => must(p.engine.jobs.at(i), `job ${i}`);
// an edit and its pause: the search's job is sent
async function edited(p: Page) {
  p.run.schedule();
  p.clock.advance(PAUSE_MS);
  await flush();
}
// the search answered with `names`, then the plan with `pick`
async function answered(p: Page, n: number, names: string[], pick?: string) {
  p.engine.answer(n, searched(...names));
  await flush();
  if (pick !== undefined) {
    p.engine.answer(n + 1, planned(pick));
    await flush();
  }
}
const sent = (p: Page) => p.engine.jobs.length;

// ---------- staleness
test("an edit during the search: the old run's late answer is dropped, the new run's is shown", async () => {
  const p = page();
  await edited(p);
  assert.equal(sent(p), 1);
  p.run.schedule(); // an edit: the job is the same, so the run under way goes on
  assert.equal(p.engine.restarts, 0, 'an edit that leaves the job as it is kills nothing');
  p.job.skills.targets.push({ group: 900 }); // a real edit
  p.run.schedule();
  assert.equal(p.engine.restarts, 1, 'a job in flight is cut short');
  const shownBefore = answer.peek();
  held(p, 0).live = true; // (as if the worker answered before the kill landed)
  p.engine.answer(0, searched('old'));
  await flush();
  assert.equal(answer.peek(), shownBefore, 'the old run draws nothing');
  assert.equal(runView.peek().searching, true, 'the new run waits out its pause');
  p.clock.advance(PAUSE_MS);
  await flush();
  await answered(p, 1, ['new'], 'pick');
  assert.equal(answer.peek().last?.r.results[0]?.career, 'new');
  assert.equal(doneOf(answer.peek()).r.choice?.career, 'pick');
  assert.equal(runView.peek().searching, false);
});

// a change of setup with places taken from the table: the setup's effect schedules, then the places' effect (its
// hand-back) schedules again in the same task (career/main.tsx)
test('two schedules in one task: one pause, one job, nothing killed', async () => {
  const p = page();
  p.run.schedule();
  p.job.career.trainees.push(1002); // (the places handed back change the job)
  p.run.schedule();
  assert.equal(p.engine.restarts, 0);
  p.clock.advance(PAUSE_MS);
  await flush();
  assert.equal(sent(p), 1, 'one job after the pause');
  assert.equal(held(p, 0).job, p.job);
  p.run.schedule(); // the same job in flight: it goes on
  p.clock.advance(PAUSE_MS);
  await flush();
  assert.equal(sent(p), 1);
  assert.equal(p.engine.restarts, 0);
});
test('an edit during the plan: its late progress and answer are dropped', async () => {
  const p = page();
  await edited(p);
  await answered(p, 0, ['best']);
  assert.equal(runView.peek().phase, 'plan');
  assert.deepEqual(held(p, 1).job.career.next, {}, "the plan job's next");
  assert.equal(held(p, 1).job.career.no_reports, true);
  p.job.skills.targets.push({ group: 901 });
  p.run.schedule();
  const before = runView.peek();
  must(held(p, 1).progress, 'its progress')({ ph: 2, k: 0, n: 3 });
  assert.equal(runView.peek(), before, "the old plan's progress writes nothing");
  const shownBefore = answer.peek();
  held(p, 1).live = true;
  p.engine.answer(1, planned('stale'));
  await flush();
  assert.equal(answer.peek(), shownBefore, 'nothing of the old run is drawn');
  assert.equal(p.engine.saves, 0, "the old plan's work is not saved");
});

test('a restart rejects a job waiting on the start: no error is shown, and the next edit can stop the next run', async () => {
  const p = page();
  p.engine.cold = true; // the first search waits on the wasm compiling
  await edited(p);
  p.job.skills.targets.push({ group: 902 });
  p.run.schedule(); // kills: the waiting job rejects with 'superseded'
  await flush();
  assert.notEqual(answer.peek().status.kind, 'error', "the superseded job's rejection is not shown");
  assert.equal(runView.peek().searching, true);
  p.engine.cold = false;
  p.clock.advance(PAUSE_MS);
  await flush();
  assert.equal(sent(p), 2);
  assert.equal(p.engine.busy(), true, 'the new run is under way');
  p.job.skills.targets.push({ group: 903 });
  p.run.schedule();
  assert.equal(p.engine.restarts, 2, 'so the next edit stops it');
});

test('a Blocked job ends the run unsearched, with its message; a thrown job error the same, as an error', async () => {
  const p = page();
  p.state.blocked = true;
  await edited(p);
  assert.equal(sent(p), 0);
  assert.equal(answer.peek().status.kind, 'empty');
  assert.equal(answer.peek().status.text, 'No target skills.');
  assert.equal(runView.peek().searching, false);
  assert.equal(p.seen.cleared, 1, 'the analysis is cleared');
  p.state.blocked = false;
  p.state.broken = true;
  await edited(p);
  assert.equal(sent(p), 0);
  assert.deepEqual(answer.peek().status, { kind: 'error', text: 'the job could not be built', act: undefined });
  assert.equal(p.seen.cleared, 2);
});
test('a failure clears the analysis; a Blocked edit under a live ticker ends it', async () => {
  const p = page();
  await edited(p);
  p.engine.answer(0, { kind: 'refusal', error: 'bad' });
  await flush();
  assert.equal(p.seen.cleared, 1);
  p.job.skills.targets.push({ group: 990 });
  await edited(p);
  assert.equal(p.clock.pending().length, 1, 'ticker live');
  p.state.blocked = true;
  await edited(p);
  assert.equal(p.clock.pending().length, 0, 'no ticker after Blocked');
});
test('a failed plan: not kept, not saved, and the same job runs again', async () => {
  const p = page();
  await edited(p);
  p.engine.answer(0, searched('best'));
  await flush();
  p.engine.answer(1, careerSearch({ results: [], next: { kind: 'refusal', error: 'no option' } }));
  await flush();
  assert.equal(answer.peek().planState.state, 'error');
  assert.equal(p.seen.saved.length, 0, 'not saved');
  p.run.schedule();
  assert.equal(runView.peek().searching, true, 'the same job runs again');
  p.clock.advance(PAUSE_MS);
  await flush();
  assert.equal(sent(p), 3, 'a new search');
});
test('an undo onto a kept answer shows it at once, with her analysis; one whose plan failed was never kept', async () => {
  const p = page();
  await edited(p);
  await answered(p, 0, ['best'], 'pick');
  // (the plan's robustness check runs in the background, and resumes on the undo: the searches are the foreground jobs)
  const searches = () => p.engine.jobs.filter((j) => !j.background).length,
    shown = p.seen.shown.length,
    jobs = searches();
  p.job.skills.targets.push({ group: 991 });
  await edited(p);
  assert.equal(searches(), jobs + 1, 'the edit searches');
  await answered(p, sent(p) - 1, ['other'], 'pick2'); // (its answer on show)
  p.job.skills.targets.pop(); // its undo: the kept answer
  await edited(p);
  assert.equal(searches(), jobs + 2, 'no search for the undo (the edit made two: its search and its plan)');
  assert.equal(answer.peek().planState.state, 'done');
  assert.equal(answer.peek().last?.r.results[0]?.career, 'best');
  assert.equal(p.seen.shown.length, shown + 2, 'her analysis is shown with it');
  // a failed plan's answer: an edit away and back runs the job again
  const q = page();
  await edited(q);
  q.engine.answer(0, searched('best'));
  await flush();
  q.engine.answer(1, careerSearch({ results: [], next: { kind: 'refusal', error: 'no option' } }));
  await flush();
  q.job.skills.targets.push({ group: 992 });
  await edited(q);
  q.job.skills.targets.pop();
  await edited(q);
  assert.equal(sent(q), 4, 'the undone job searches again');
});

test('the engine failing or refusing: the error is shown and the run ends', async () => {
  const p = page();
  await edited(p);
  held(p, 0).live = false;
  held(p, 0).reject(new Error('the solver failed to start'));
  await flush();
  assert.equal(answer.peek().status.text, 'the solver failed to start');
  assert.equal(runView.peek().searching, false);
  p.job.skills.targets.push({ group: 904 });
  await edited(p);
  p.engine.answer(1, { kind: 'refusal', error: 'bad job' });
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
  assert.equal(p.clock.pending().length, 1, 'the pause');
  p.run.schedule(); // the same job: nothing new
  p.job.skills.targets.push({ group: 905 });
  p.run.schedule(); // a new edit replaces the pause
  assert.equal(p.clock.pending().length, 1);
  p.clock.advance(PAUSE_MS);
  await flush();
  assert.equal(p.clock.pending().length, 1, 'the ticker');
  const paints = p.seen.paints;
  p.clock.advance(3000);
  assert.equal(p.seen.paints - paints, 3, 'a repaint each second');
  await answered(p, 0, ['best']);
  p.clock.advance(1000);
  assert.equal(p.clock.pending().length, 1, 'one ticker through the plan phase');
  p.engine.answer(1, planned('pick'));
  await flush();
  assert.equal(p.clock.pending().length, 0, 'no ticker after the answer');
  const done = p.seen.paints;
  p.clock.advance(5000);
  assert.equal(p.seen.paints, done, 'no repaint after');
  // an edit during a search, then the search: one ticker at a time
  p.job.skills.targets.push({ group: 906 });
  await edited(p);
  p.job.skills.targets.push({ group: 907 });
  await edited(p);
  assert.equal(p.clock.pending().length, 1, 'the killed run left no ticker');
});

// ---------- the comparison follows the runs
test('every new run makes the comparison stale before anything awaits (the analysis keeps no run id of its own)', async () => {
  const p = page();
  p.run.schedule();
  assert.deepEqual([p.seen.stops, p.seen.outdates], [1, 0], 'an edit stops it');
  p.clock.advance(PAUSE_MS);
  assert.deepEqual([p.seen.stops, p.seen.outdates], [1, 1], 'the search outdates it, synchronously');
  p.run.schedule(); // the same job: no new run, nothing goes stale
  assert.deepEqual([p.seen.stops, p.seen.outdates], [1, 1]);
  // a run without an edit, the page load's first: it outdates the comparison too
  const q = page();
  await q.run.start();
  assert.deepEqual([q.seen.stops, q.seen.outdates], [0, 1]);
});

// ---------- the check through the machine
test("a run's check sends every variant through its own writes, and a new plan stops it", async () => {
  const p = page();
  await edited(p);
  await answered(p, 0, ['best'], 'pick');
  const made = answer.peek().planState;
  assert.equal(made.state, 'done');
  assert.equal(p.engine.saves, 1, "a plan made: the worker's work is kept");
  assert.equal(sent(p), 3, 'the first check job');
  assert.deepEqual(held(p, 2).job.career.next, { robust: 'next', pair_search: true });
  assert.equal(held(p, 2).background, true);
  for (const [n, reply] of [check(2), check(1), check(0)].entries()) {
    p.engine.answer(2 + n, reply);
    await flush();
  }
  assert.equal(doneOf(answer.peek()).id, made.id, 'the same plan, its record replaced by each step');
  assert.equal(sent(p), 6, 'the pair search after the last variant');
  // an edit, and its run ends on a new plan: the check's late reply writes nothing
  p.job.skills.targets.push({ group: 908 });
  await edited(p);
  await answered(p, 6, ['best'], 'pick');
  const fresh = doneOf(answer.peek());
  held(p, 5).live = true;
  p.engine.answer(5, check(0, { trainees: [] }));
  await flush();
  assert.equal(doneOf(answer.peek()).r.robust, undefined, "the old plan's check did not write the new one");
  assert.equal(doneOf(answer.peek()).id, fresh.id);
});

test("the engine restarted under a check: the check's race ends with the kill, and a new run's edit leaves it unmarked", async () => {
  // no edit: the kill loses the check, which is marked unavailable and drawn
  const alone = page();
  await edited(alone);
  await answered(alone, 0, ['best'], 'pick');
  const paints = alone.seen.paints;
  alone.engine.restart();
  await flush();
  assert.equal(doneOf(answer.peek()).r.check_unavailable, true);
  assert.equal(alone.seen.paints - paints, 1, 'the status line follows the check');
  // an edit first: the check is stale and marks nothing
  const p = page();
  await edited(p);
  await answered(p, 0, ['best'], 'pick');
  assert.equal(sent(p), 3);
  p.job.skills.targets.push({ group: 909 });
  p.run.schedule(); // the check's job is background: the engine is not busy, so nothing is killed
  assert.equal(p.engine.restarts, 0);
  p.engine.restart(); // (a stopped comparison kills the worker)
  await flush();
  const shown = answer.peek().planState;
  assert.notEqual(shown.state === 'done' && shown.r.check_unavailable, true, 'a stale check marks nothing');
});

// ---------- the answers kept and the reload
test('an edit and its undoing: the kept answer is drawn again with no job, and its unfinished check resumes', async () => {
  const p = page();
  await edited(p);
  await answered(p, 0, ['best'], 'pick');
  p.engine.answer(2, check(1));
  await flush();
  // another job, answered (its answer on show), then a third under way (its ticker running)
  p.job.skills.targets.push({ group: 910 });
  await edited(p);
  await answered(p, sent(p) - 1, ['other'], 'pick');
  p.job.skills.targets.push({ group: 911 });
  await edited(p);
  const jobs = sent(p);
  // back to the first job: drawn from the answers kept
  p.job.skills.targets.splice(1);
  p.run.schedule();
  p.clock.advance(PAUSE_MS);
  await flush();
  assert.equal(answer.peek().last?.r.results[0]?.career, 'best');
  assert.equal(doneOf(answer.peek()).r.robust?.pending, 1, 'the kept answer, with its check as far as it ran');
  assert.equal(runView.peek().searching, false);
  assert.equal(sent(p), jobs + 1, 'no search: one job, its check resumed');
  assert.equal(held(p, -1).background, true);
  assert.equal(p.clock.pending().length, 0, "the third run's ticker ends with it");
});

test('a reload: the answer kept for this setup and these fingerprints is drawn at once, its check resumes', async () => {
  const first = page();
  await edited(first);
  await answered(first, 0, ['best'], 'pick');
  const { key, value } = must(first.seen.saved[0], 'the answer saved');
  assert.ok(key.startsWith(`${stateKey()}\n`), "the saved answer's key carries the fingerprints");
  // the next visit, the same setup (made here on another page's job: the first page's answer is in the cache too)
  const again = page(),
    savedFor = (p: Page, fingerprints = stateKey()) => {
      const saved = { key: `${fingerprints}\n${jobKey(p.job)}\n${p.deps.page.places()}`, value };
      return async () => saved;
    };
  again.deps.page.loadAnswer = savedFor(again);
  await again.run.start();
  await flush();
  assert.equal(again.engine.jobs.filter((j) => !j.background).length, 0, 'nothing searched');
  assert.equal(answer.peek().last?.r.results[0]?.career, 'best');
  assert.notEqual(doneOf(answer.peek()).id, doneOf(value).id, 'the saved plan gets an id of this page');
  assert.equal(again.engine.jobs.length, 1, 'its check resumes');
  // another solver or data release: the saved answer is not taken
  const other = page();
  other.deps.page.loadAnswer = savedFor(other, 'wasm0|engine0'); // (kept by an older release)
  await other.run.start();
  await flush();
  assert.equal(other.engine.jobs.length, 1, 'a search is sent');
  assert.equal(held(other, 0).background, false);
});

test("Compare's kept plan: the plan phase takes it and sends no plan job", async () => {
  const p = page();
  const jp = { ...p.job, career: { ...p.job.career, next: {}, no_reports: true } };
  store.keepComparisonPlan(jp, plan('kept'), 7);
  await edited(p);
  p.engine.answer(0, searched('best'));
  await flush();
  assert.equal(doneOf(answer.peek()).r.choice?.career, 'kept');
  assert.equal(doneOf(answer.peek()).ms, 7);
  assert.equal(p.engine.jobs.filter((j) => !j.background).length, 1, 'only the search was sent');
  // its check runs, and each step reaches the kept plan too: a run that takes it again finds the check as far as it ran
  p.engine.answer(1, check(1, null, 'kept'));
  await flush();
  assert.deepEqual(store.keptPlan(planKey(jp))?.r.robust, check(1, null, 'kept').robust);
});

test('a trainee chosen by hand: no plan phase, the best odds shown', async () => {
  const p = page({ trainee: 1001 });
  await edited(p);
  p.engine.answer(0, searched('mine'));
  await flush();
  assert.equal(answer.peek().planState.state, 'none');
  assert.equal(sent(p), 1);
  assert.equal(p.seen.shown.at(-1)?.career, 'mine');
});

// review-7 P1: Compare's kept plan whose check an edit cut short (its record holds the variants run so far): a run that
// takes it resumes the check, as a kept answer's does; one whose check is done, or unavailable, sends none
test("Compare's kept plan with its check cut short: the run that takes it resumes the check", async () => {
  const p = page();
  const jp = { ...p.job, career: { ...p.job.career, next: {}, no_reports: true } },
    kept = plan('kept');
  store.keepComparisonPlan(jp, { ...kept, robust: check(1, null, 'kept').robust }, 7);
  await edited(p);
  p.engine.answer(0, searched('best'));
  await flush();
  assert.equal(doneOf(answer.peek()).r.robust?.pending, 1);
  assert.equal(p.engine.jobs.length, 2, 'a check job');
  assert.equal(held(p, 1).background, true);
  // its check done, or unavailable: nothing more is sent
  for (const r of [
    { ...kept, robust: check(0, { trainees: [] }, 'kept').robust },
    { ...kept, check_unavailable: true },
  ]) {
    const q = page();
    store.keepComparisonPlan({ ...q.job, career: { ...q.job.career, next: {}, no_reports: true } }, r, 7);
    await edited(q);
    q.engine.answer(0, searched('best'));
    await flush();
    assert.equal(q.engine.jobs.length, 1, 'only the search');
  }
});
