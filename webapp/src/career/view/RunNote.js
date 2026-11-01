// The status line: the phase of the run at work, or its answer's status and actions.
import { $, el } from '../../dom.js';
import { runnerHTML, setRunner } from '../../images.js';
import { plan } from '../../vets.js';
import { answer, runView } from '../state.ts';
import { planCaveat } from '../../proofs.mjs';
import { shown } from '../job.js';
import { btn } from './parts.js';

export function paint() {
  const note = $('run-note'),
    { searching, started } = runView.value,
    { status } = answer.value;
  setRunner(plan.trainee || shown()?.trainee?.id); // the runner: the trainee you chose, else the career on show
  $('rhead').dataset.kind = searching ? 'busy' : status.kind;
  const dimmed = searching && started && !!shown(); // the previous answer, on screen until this run's
  $('results').classList.toggle('stale', dimmed || (!searching && ['error', 'empty'].includes(status.kind)));
  $('tree').classList.toggle('stale', dimmed);
  $('results').setAttribute('aria-busy', String(searching));
  if (searching) paintBusy(note);
  else {
    note.textContent = status.text + (status.kind === 'done' ? planNote() : '');
    // its actions, e.g. Import veterans and Try sample veterans (the last is #run-act)
    const acts = status.act ? [status.act].flat() : [];
    acts.forEach((a, i) => {
      const b = btn(a.label, i === acts.length - 1 ? 'primary' : '', a.run);
      if (i === acts.length - 1) b.id = 'run-act';
      note.append(' ', b);
    });
  }
}
// the status line during a run: the search's lead (in the plan phase), the phase at work, the plan's progress and the
// phase's seconds so far
function paintBusy(note) {
  const { phase, phaseStart, lead, progress } = runView.value,
    seconds = Math.floor((performance.now() - phaseStart) / 1000);
  note.textContent = phase === 'plan' && lead ? `${lead} ` : '';
  const line = el('span');
  line.innerHTML = runnerHTML();
  line.append(
    phase === 'plan' ? 'Planning your next career…' : 'Searching careers…',
    el('span', '', seconds >= 1 ? ` ${seconds} s` : ''),
  );
  /** @type {HTMLElement} */
  (line.lastChild).setAttribute('aria-hidden', 'true');
  // (the plan's progress, before the seconds; like them, not read out at every step)
  if (phase === 'plan' && progress) {
    const pg = el('span', 'runprog', ` (${progress})`);
    pg.setAttribute('aria-hidden', 'true');
    line.insertBefore(pg, line.lastChild);
  }
  note.appendChild(line);
}
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
