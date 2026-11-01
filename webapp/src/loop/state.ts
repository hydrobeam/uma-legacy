// ---------- the loop page's run state: what its status line, results and Export show, three records in signals ----------
// `loopRun`: the search as the status line draws it (running or about to, its phase, its clock). `loopView`: the status
// line when no search runs, and the result on show. `loopDrawn`: the result the results column draws, and the targets
// it was searched for (the machine draws a result only when its loops differ from those on show). All are handed out
// read-only; the run machine (run.ts) changes
// them only through the transitions below, each one write (a batch when it changes both), so an effect wakes once per
// transition. What nothing draws stays in the machine: its staleness token (`searchId`), `busy` and its timers.
// `resultsStale` and `headline` derive from the two.
import { batch, computed } from '@preact/signals';
import type { ReadonlySignal } from '@preact/signals';
import { fmtIn } from '../dom.ts';
import { deepFreeze, store } from '../store.ts';
import type { DeepReadonly } from '../store.ts';
import type { JobTarget } from './job.ts';
import type { DrawnReport, ShownResult, Status } from './types.ts';

/** 'check': the fast result is on screen and the exact check is running */
export type Phase = 'search' | 'check';
export interface RunState {
  /** a search runs, or is about to (the pause after a change) */
  searching: boolean;
  phase: Phase;
  /** when the search (or its pause) started, and the clock now (performance.now ms; `now` ticks each second) */
  startedAt: number;
  now: number;
}
export interface LoopView {
  status: Status;
  lastResult: ShownResult | null;
}
export interface Drawn {
  r: DrawnReport;
  targets: readonly JobTarget[];
  /** the focus was inside the result it replaces (an open panel's Owned switch, say): focus the redraw removes goes to
   *  the results column, not the page top */
  refocus: boolean;
}

const IDLE: RunState = { searching: false, phase: 'search', startedAt: 0, now: 0 };
const LOADING: LoopView = { status: { kind: 'loading', text: '' }, lastResult: null };
// The records are frozen, their status too; the report is the solver's answer as it came (a large tree nothing writes),
// read-only by type only.
const freezeView = (view: DeepReadonly<LoopView>): DeepReadonly<LoopView> =>
  Object.freeze({
    status: deepFreeze(view.status),
    lastResult:
      view.lastResult && Object.freeze({ ...view.lastResult, meta: Object.freeze({ ...view.lastResult.meta }) }),
  });
const [runRecord, setRun] = store<RunState>(Object.freeze(IDLE));
const [viewRecord, setView] = store<LoopView>(freezeView(LOADING));
const [drawnRecord, setDrawn] = store<Drawn | null>(null);
export const loopRun: ReadonlySignal<DeepReadonly<RunState>> = runRecord;
export const loopView: ReadonlySignal<DeepReadonly<LoopView>> = viewRecord;
export const loopDrawn: ReadonlySignal<DeepReadonly<Drawn> | null> = drawnRecord;
/** back to the page's first state (a test's start) */
export function initLoopState() {
  batch(() => {
    setRun(Object.freeze(IDLE));
    setView(freezeView(LOADING));
    setDrawn(null);
  });
}

// the result on show is from other settings: a change not searched yet (or under way), a stopped or failed search
const STALE: readonly Status['kind'][] = ['error', 'empty', 'stopped', 'pending'];
export const resultsStale = computed(() => runRecord.value.searching || STALE.includes(viewRecord.value.status.kind));
/** the top result's odds ("1 in 139"), for the phone's floating button; '' with no result on show */
export const headline = computed(() => {
  const shown = viewRecord.value.lastResult;
  return shown ? fmtIn(shown.r.top[0].loop.p_all) : '';
});

// ---------- transitions
/** a search runs, or is about to (the pause after a change): its clock starts at `at` */
export const beginSearch = (at: number) => {
  setRun(Object.freeze({ searching: true, phase: 'search', startedAt: at, now: at }));
};
/** the clock of the running search */
export const tick = (at: number) => {
  setRun(Object.freeze({ ...runRecord.peek(), now: at }));
};
/** the fast result, on show while the exact check runs */
export const interim = (result: ShownResult, at: number) =>
  batch(() => {
    setView(freezeView({ ...viewRecord.peek(), lastResult: result }));
    setRun(Object.freeze({ ...runRecord.peek(), phase: 'check', now: at }));
  });
/** no search runs: the status line says `status`; `result`, when given, is the result on show now */
export const settle = (status: Status, result?: ShownResult) =>
  batch(() => {
    setView(freezeView({ status, lastResult: result ?? viewRecord.peek().lastResult }));
    setRun(Object.freeze({ ...runRecord.peek(), searching: false, phase: 'search' }));
  });
/** the results column draws `r` (searched for `targets`); `refocus`: Drawn's */
export const draw = (r: DrawnReport, targets: readonly JobTarget[], refocus: boolean) => {
  setDrawn(Object.freeze({ r, targets, refocus }));
};
