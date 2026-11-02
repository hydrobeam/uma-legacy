// ---------- the loop search's run machine: its scheduling, the search, Stop, and the result shown first ----------
// The search runs by itself: a change of setup schedules one after a pause (PAUSE_MS, so a run of clicks is one search);
// a change during a search kills the worker and starts again (a fresh worker is ready in ~60 ms). With Auto-search off,
// a change leaves the results on show, marked out of date, until the Search button.
// What the page draws is state.ts's, changed by its transitions. The machine's own state is in createRun's closure:
// `searchId` (the newest search's number: an older search's answers, interim or final, are dropped), `busy` (a search
// runs in the engine: a kill cuts it short), the pause timer and the clock's ticker, what is on show (`shown`, so an
// answer with the same loops is not drawn again) and the finished search of it (`done`: an automatic search of the same
// job shows that again instead of running). Everything outside, the engine, the clock, the timers, storage, the
// settings, the store's transitions and what draws a result, comes in `deps`, made once by main.tsx: a model module, it
// reads no store itself (.dependency-cruiser.mjs checks it).
import { ERA } from '../saved.ts';
import { readJson, update } from '../storage.ts';
import type { Backing } from '../storage.ts';
import type { SettingsView } from '../settings.ts';
import type { SolverAnswer, SolverRefusal } from '../solver.ts';
import { blockedStatus, buildJob, doneText, jobKey } from './job.ts';
import type { Built, DeckJob, JobTarget } from './job.ts';
import type * as LoopState from './state.ts';
import type { DrawnReport, LoopReport, ShownResult, Status } from './types.ts';

/** the run store's transitions and the run state the machine reads (state.ts) */
export type LoopStore = Pick<typeof LoopState, 'loopRun' | 'beginSearch' | 'interim' | 'settle' | 'tick'>;
/** the pause after a change before the automatic search (ms) */
export const PAUSE_MS = 550;
export type Timer = number;
export type DeckAnswer = SolverAnswer<LoopReport>;
export interface LoopDeps {
  /** the engine (engine.ts `run`): the job's answer; `interim` gets its fast first pass. A job killed while it runs never
   *  answers; one killed while it waits for the worker to start rejects (`superseded`), so a caught error may be stale */
  run: (job: DeckJob, interim: (answer: DeckAnswer) => void) => Promise<DeckAnswer>;
  /** kill the worker (the job it runs never answers; one waiting for its start rejects) and start a fresh one
   *  (engine.ts `restartEngine`) */
  restart: () => void;
  /** the clock (ms) */
  now: () => number;
  setTimer: (run: () => void, ms: number) => Timer;
  /** a timer that fires every `ms`; clearTimer stops either kind (the browser's two share one list) */
  every: (run: () => void, ms: number) => Timer;
  clearTimer: (timer: Timer | undefined) => void;
  /** where the last finished result is kept */
  storage: () => Backing;
  /** the run store (state.ts): its transitions, and the run state read with .peek() */
  store: LoopStore;
  /** the settings now (read when a search or its pause starts) */
  settings: () => SettingsView;
  /** draws a result (Results.tsx `drawResult`) */
  render: (r: DrawnReport, targets: JobTarget[]) => void;
  /** the "no targets" status's button (presets.ts `loadExample`) */
  loadExample: () => void;
  /** the precomputed example and the settings it is the result of (the defaults); none in the Grand Masters era */
  example: { r: LoopReport; settings: SettingsView } | null;
}
/** what showFirst put on show: the kept result of these settings (done), the example, or nothing */
export type First = 'kept' | 'example' | null;

/** a report with loops: not a refusal, not empty */
const hasLoops = (r: LoopReport | SolverRefusal): r is DrawnReport => r.kind === 'loops' && r.top?.length > 0;
// the last finished result, per era (its key holds the era's data fingerprint)
const lastResultKey = () => `llp:last:${ERA}`;

export function createRun(deps: LoopDeps) {
  const { loopRun, beginSearch, interim, settle, tick } = deps.store;
  let searchId = 0,
    busy = false;
  let pauseTimer: Timer | undefined, ticker: Timer | undefined;
  let shown: { key: string | null; r: string | null; example: boolean } = { key: null, r: null, example: false };
  let done: { key: string; status: Status } | null = null;

  function stopEngine() {
    deps.restart();
    busy = false;
  }
  // no search runs any more: no clock, the given status (and the result now on show, when one came)
  function endSearch(status: Status, result?: ShownResult) {
    deps.clearTimer(ticker);
    settle(status, result);
  }
  // Stop: the running search (or the one about to start) is dropped and its worker killed; the results on screen stay
  function stopSearch() {
    if (!loopRun.peek().searching) return;
    searchId++;
    deps.clearTimer(pauseTimer);
    if (busy) stopEngine();
    const checking = loopRun.peek().phase === 'check';
    const again = { label: 'Search again', run: () => void doSearch() };
    endSearch(
      checking
        ? {
            kind: 'done',
            text: 'Stopped the exact check: the result below is ranked by odds with targets treated as independent.',
            act: again,
          }
        : { kind: 'stopped', text: 'Stopped. The results below are from before your change.', act: again },
    );
  }
  // A change of setup: with Auto-search on, a search after the pause (a change during the pause or the search restarts
  // it); off, the results stay, marked out of date, until the Search button
  function scheduleSearch() {
    deps.clearTimer(pauseTimer);
    if (deps.settings().auto === false) {
      if (loopRun.peek().searching) stopSearch();
      settle({
        kind: 'pending',
        text: 'Settings changed: press Search.',
        act: { label: 'Search', run: () => void doSearch() },
      });
      return;
    }
    if (busy) {
      searchId++;
      stopEngine();
    } // a search for the old settings is pointless: stop it now, not when the next starts
    pauseTimer = deps.setTimer(() => void doSearch(true), PAUSE_MS);
    // "Searching…" during the pause, but the seconds count only once the search runs (doSearch starts the clock)
    deps.clearTimer(ticker);
    beginSearch(deps.now());
  }
  // auto: the search the pause starts (not the Search buttons): edits that end where they started run nothing when the
  // result on show is the finished search of the same job (its key holds the solver's and data's fingerprints, the era
  // and every setting the job carries)
  async function doSearch(auto = false) {
    const thisSearch = ++searchId;
    deps.clearTimer(pauseTimer);
    // Phase 1: the job, or why there is none (an unexpected error in building it shows as one)
    let built: Built | { error: string };
    try {
      built = buildJob(deps.settings());
    } catch (e) {
      built = { error: e instanceof Error ? e.message : String(e) };
    }
    if (!('job' in built)) {
      if (busy) stopEngine();
      endSearch('error' in built ? { kind: 'error', text: built.error } : blockedStatus(built, deps.loadExample));
      return;
    }
    const { job, targets } = built;
    const key = jobKey(job);
    if (auto === true && !busy && done && key === done.key && shown.key === done.key && !shown.example) {
      endSearch(done.status);
      return;
    }
    // Phase 2: the search, its clock ticking each second
    if (busy) stopEngine(); // a search is still running: stop it and start over
    beginSearch(deps.now());
    deps.clearTimer(ticker);
    ticker = deps.every(() => tick(deps.now()), 1000);
    // a result is drawn unless the same loops and decks are already on screen
    const draw = (r: DrawnReport) => {
      const loops = JSON.stringify(r.top);
      if (shown.example || shown.key !== key || shown.r !== loops) deps.render(r, targets);
      shown = { key, r: loops, example: false };
      return { kind: 'deck', r, meta: {} } as const;
    };
    let status: Status, result: ShownResult | undefined;
    try {
      busy = true;
      const answer = await deps.run(job, (fast) => {
        if (thisSearch !== searchId || !hasLoops(fast.r)) return;
        interim(draw(fast.r), deps.now());
      });
      if (thisSearch !== searchId) return; // superseded: a newer search owns the page
      busy = false;
      // Phase 3: the answer, kept for a reload
      if (!hasLoops(answer.r))
        status = {
          kind: 'error',
          text:
            answer.r.kind === 'refusal' ? `Not searched: ${answer.r.error}` : 'No result: no loop fits these settings.',
        };
      else {
        result = draw(answer.r);
        status = doneText(answer.r, answer.ms);
        done = { key, status };
        // (the whole entry is this result's: storage blocked or full, it is not kept for a reload)
        update(lastResultKey(), () => JSON.stringify({ key, ms: answer.ms, r: answer.r }), deps.storage);
      }
    } catch (e) {
      if (thisSearch !== searchId) return;
      busy = false;
      status = { kind: 'error', text: e instanceof Error ? e.message : String(e) };
    }
    endSearch(status, result);
  }
  // What to show before the first search finishes: the kept result of these exact settings (done: nothing to search),
  // else the precomputed example when the settings are the example's, else nothing (the example under other settings
  // would show a result that isn't theirs)
  function showFirst(): First {
    let built: Built;
    try {
      built = buildJob(deps.settings());
    } catch (e) {
      reportError(e); // a bug (doSearch shows it as the search's error): the page goes on with nothing on show
      return null;
    }
    if (!('job' in built)) return null;
    const key = jobKey(built.job);
    // (storage blocked or the kept result unreadable: none to show)
    const kept = readJson(lastResultKey(), deps.storage) as { key?: string; ms: number; r?: LoopReport | null } | null;
    if (kept?.key === key && kept.r?.top && hasLoops(kept.r)) {
      shown = { key, r: JSON.stringify(kept.r.top), example: false };
      done = { key, status: doneText(kept.r, kept.ms) };
      deps.render(kept.r, built.targets);
      settle(done.status, { kind: 'deck', r: kept.r, meta: {} });
      return 'kept';
    }
    const { example } = deps;
    if (!example || !hasLoops(example.r)) return null;
    const exampleJob = buildJob(example.settings);
    if (!('job' in exampleJob) || jobKey(exampleJob.job) !== key) return null;
    shown = { key, r: null, example: true };
    deps.render(example.r, built.targets);
    settle(
      { kind: 'example', text: 'Showing an example: Unity Cup skills, everything owned' },
      { kind: 'deck', r: example.r, meta: { example: true } },
    );
    return 'example';
  }
  return { scheduleSearch, doSearch, stopSearch, showFirst };
}
