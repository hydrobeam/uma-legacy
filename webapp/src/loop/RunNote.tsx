// ---------- the status line heading the results, and its copy in the phone's floating button ----------
// While a search runs: its time and a Stop button; otherwise the status (state.ts loopView) and the button it offers.
// The button is keyed by what it does, so a status that offers the same one keeps the element, and its focus. Whenever
// the results on screen are not those of the current settings (an error, an empty state, a stopped or running search)
// they stay, dimmed, instead of being wiped: main.tsx marks the template's elements around (`pillText` is the phone's
// floating button's text).
import type { ComponentChildren } from 'preact';
import { $ } from '../dom.js';
import { must } from '../must.ts';
import { Runner } from '../ui/Images.tsx';
import { loopDrawn, loopRun, loopView } from './state.ts';
import type { RunState } from './state.ts';
import type { Status } from './types.ts';
import type { DeepReadonly } from '../store.ts';

const seconds = (run: DeepReadonly<RunState>) => Math.floor((run.now - run.startedAt) / 1000);
const doing = (run: DeepReadonly<RunState>) => (run.phase === 'check' ? 'Checking the exact odds…' : 'Searching…');
// the seconds, hidden from screen readers: the live region would read them out every second
const timeText = (run: DeepReadonly<RunState>) => (seconds(run) >= 1 ? ` ${seconds(run)} s` : '');

/** @param stop the Stop button's action (run.ts stopSearch) */
export function RunNote({ stop }: { stop: () => void }) {
  const run = loopRun.value,
    status = loopView.value.status;
  let note: ComponentChildren, act: { key: string; label: string; run: () => void } | null;
  if (run.searching) {
    // one line beside the runner: the exact check's caveat is its tooltip (and read to screen readers)
    const caveat =
      run.phase === 'check' ? (
        <span class="vh"> The result below may still change.</span>
      ) : seconds(run) >= 1 && loopDrawn.value ? (
        '. The results below are from before your change.'
      ) : null;
    note = (
      <>
        <Runner />
        <span>
          {doing(run)}
          <span aria-hidden="true">{timeText(run)}</span>
          {caveat}
        </span>
      </>
    );
    act = { key: 'stop', label: 'Stop', run: stop };
  } else {
    note = status.text || null;
    const given = status.act;
    act = given
      ? {
          key: given.click || given.label,
          label: given.label,
          run: given.run || (() => $(must(given.click, 'an act without run names the button it presses')).click()),
        }
      : null;
  }
  return (
    <>
      <p
        class={run.searching ? 'runnote busy' : 'runnote'}
        id="run-note"
        role="status"
        aria-live="polite"
        title={
          run.searching && run.phase === 'check'
            ? 'The result below is ranked by quick odds; the exact check can still reorder it'
            : ''
        }
      >
        {note}
      </p>
      <span class="ract" id="r-act">
        {act && (
          <button key={act.key} type="button" class="btn" onClick={act.run}>
            {act.label}
          </button>
        )}
      </span>
    </>
  );
}
/** the phone's floating status button: the search and its time, or the top result's odds, or why there is none */
export function pillText(run: DeepReadonly<RunState>, status: DeepReadonly<Status>, top: string) {
  if (run.searching) return doing(run) + timeText(run);
  if (status.kind === 'done' || status.kind === 'example') return top ? `${top} · see result ↑` : status.text;
  return status.kind === 'loading' ? status.text : 'Not searched · see why ↑';
}
/** whether the status line shows anything (the floating button hides with it) */
export const noteShown = (run: DeepReadonly<RunState>, status: DeepReadonly<Status>) => run.searching || !!status.text;
