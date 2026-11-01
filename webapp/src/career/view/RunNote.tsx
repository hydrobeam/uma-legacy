// The status line (in #run-note, the layout's live region, there from the first paint): the phase of the run at work, or its
// answer's status and actions. Its buttons are keyed by what they say, so a status that offers the same one keeps the
// element, and its focus. The phase's seconds come from `clock`, which the run machine's ticker moves (`tickNote`).
import { Fragment } from 'preact';
import { signal } from '@preact/signals';
import { Runner } from '../../ui/Images.tsx';
import { planCaveat } from '../../proofs.ts';
import { answer, runView } from '../state.ts';
import { Button } from '../../ui/Button.tsx';

const clock = signal(0);
/** the status line's clock: now (the run machine calls it each second while a job runs, and after each change) */
export const tickNote = () => {
  clock.value = performance.now();
};
// the plan's line: its time and the named assumptions its searches rest on (the job's, and its proven rows' own) that
// the search's line does not state (`planCaveat`), or why it failed
function planNote() {
  const { last, planState } = answer.value;
  if (planState.state === 'error') return ` Plan not made: ${planState.text}`;
  if (planState.state !== 'done') return '';
  const plan = planState.r,
    assumes = [
      ...(plan.job_assumes || []),
      ...plan.options.filter((o) => o.exact && o.proven !== false).flatMap((o) => o.assumes || []),
    ];
  return ` Planned in ${(planState.ms / 1000).toFixed(1)} s.${planCaveat(assumes, last?.r || {})}`;
}
// during a run: the search's lead (in the plan phase), the phase at work, the plan's progress and the phase's seconds so
// far (the progress and the seconds not read out at every step)
function Busy() {
  const { phase, phaseStart, lead, progress } = runView.value,
    seconds = Math.floor((clock.value - phaseStart) / 1000);
  return (
    <>
      {phase === 'plan' && lead ? `${lead} ` : ''}
      {/* the runner beside the phase, one line */}
      <span class="inline-flex items-center gap-1.5">
        <Runner game />
        {phase === 'plan' ? 'Planning your next career…' : 'Searching careers…'}
        {phase === 'plan' && progress && <span aria-hidden="true">{` (${progress})`}</span>}
        <span aria-hidden="true">{seconds >= 1 ? ` ${seconds} s` : ''}</span>
      </span>
    </>
  );
}
export function RunNote() {
  const { searching } = runView.value,
    { status } = answer.value;
  if (searching) return <Busy />;
  // its actions, e.g. Import veterans and Try sample veterans (the last is #run-act)
  const acts = status.act ? [status.act].flat() : [];
  return (
    <>
      {status.text + (status.kind === 'done' ? planNote() : '')}
      {acts.map((a, i) => {
        const last = i === acts.length - 1;
        return (
          <Fragment key={a.label}>
            {' '}
            <Button variant={last ? 'primary' : 'plain'} id={last ? 'run-act' : undefined} onClick={a.run}>
              {a.label}
            </Button>
          </Fragment>
        );
      })}
    </>
  );
}
