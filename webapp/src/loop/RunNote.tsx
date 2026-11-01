// ---------- the status line heading the results, and its copy in the phone's floating button ----------
// While a search runs: its time and a Stop button; otherwise the status (state.ts loopView) and the button it offers.
// The button is keyed by what it does, so a status that offers the same one keeps the element, and its focus. Whenever
// the results on screen are not those of the current settings (an error, an empty state, a stopped or running search)
// they stay, dimmed, instead of being wiped: main.tsx's App marks the layout's elements around (`pillText` is the
// phone's floating button's text). Two parts, both in the layout from the first paint: `RunNote` draws the text in the
// live region #run-note, `RunAct` the button in #r-act.
import { must } from '../must.ts';
import { ownAllUmas } from '../roster.ts';
import { Runner } from '../ui/Images.tsx';
import { loopDrawn, loopRun, loopView } from './state.ts';
import type { RunState } from './state.ts';
import type { Status } from './types.ts';
import type { DeepReadonly } from '../store.ts';

const seconds = (run: DeepReadonly<RunState>) => Math.floor((run.now - run.startedAt) / 1000);
const doing = (run: DeepReadonly<RunState>) => (run.phase === 'check' ? 'Checking the exact odds…' : 'Searching…');
// the seconds, hidden from screen readers: the live region would read them out every second
const timeText = (run: DeepReadonly<RunState>) => (seconds(run) >= 1 ? ` ${seconds(run)} s` : '');

/** the status line's text, drawn in the layout's live region (#run-note, a polite status that exists from load, so
 *  its first text is a change the screen reader announces); main.tsx marks the region itself (busy, its tooltip) */
export function RunNote() {
  const run = loopRun.value;
  if (!run.searching) return <>{loopView.value.status.text || null}</>;
  // one line beside the runner: the exact check's caveat is its tooltip (and read to screen readers)
  const caveat =
    run.phase === 'check' ? (
      <span class="vh"> The result below may still change.</span>
    ) : seconds(run) >= 1 && loopDrawn.value ? (
      '. The results below are from before your change.'
    ) : null;
  return (
    <>
      <Runner />
      <span>
        {doing(run)}
        <span aria-hidden="true">{timeText(run)}</span>
        {caveat}
      </span>
    </>
  );
}
// the buttons an act names by id (loop/job.ts `blockedStatus`), by what they do: `#r-all` (ui/Roster.tsx), Mark all
// umas owned
const PRESSES: Partial<Record<string, () => void>> = { 'r-all': ownAllUmas };
/** the button beside the status line (#r-act): Stop while a search runs, else the one the status offers
 *  @param stop the Stop button's action (run.ts stopSearch) */
export function RunAct({ stop }: { stop: () => void }) {
  const run = loopRun.value;
  const given = run.searching ? null : loopView.value.status.act;
  const act = run.searching
    ? { key: 'stop', label: 'Stop', run: stop }
    : given
      ? {
          key: given.click || given.label,
          label: given.label,
          // (an act without `run` names the button whose action it takes: PRESSES)
          run: given.run || must(PRESSES[given.click ?? ''], 'an act without run names a button PRESSES has'),
        }
      : null;
  return (
    act && (
      <button key={act.key} type="button" class="btn" onClick={act.run}>
        {act.label}
      </button>
    )
  );
}
/** the status line's tooltip: while the exact check runs, why the result may still change */
export const noteTitle = (run: DeepReadonly<RunState>) =>
  run.searching && run.phase === 'check'
    ? 'The result below is ranked by quick odds; the exact check can still reorder it'
    : '';
/** the phone's floating status button: the search and its time, or the top result's odds, or why there is none */
export function pillText(run: DeepReadonly<RunState>, status: DeepReadonly<Status>, top: string) {
  if (run.searching) return doing(run) + timeText(run);
  if (status.kind === 'done' || status.kind === 'example') return top ? `${top} · see result ↑` : status.text;
  return status.kind === 'loading' ? status.text : 'Not searched · see why ↑';
}
/** whether the status line shows anything (the floating button hides with it) */
export const noteShown = (run: DeepReadonly<RunState>, status: DeepReadonly<Status>) => run.searching || !!status.text;
