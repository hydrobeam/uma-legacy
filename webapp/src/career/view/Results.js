// The results column (#results): the options table (options.js) and Try a run (analysis.js), still built with el()
// (steps 9 and 11), so drawn again by the entry after each change.
import { $ } from '../../dom.js';
import { markOwn } from '../../popover.js';
import { places } from '../../veterans/store.ts';
import { optionsEl } from '../options.js';
import { renderSimulation } from '../analysis.js';
import { answer } from '../state.ts';
import { must } from '../../must.ts';
import { refFor, shown } from '../job.js';

export function renderResults() {
  const box = $('results');
  box.textContent = '';
  const r = shown();
  if (!r) {
    renderSimulation();
    return;
  }
  // the table: the plan's careers, or the search's with a place open; with every place chosen, the last plan's (refFor)
  const plan = places.value,
    open = !plan.trainee || plan.parents.some((p) => !p),
    { last, planState } = answer.value;
  if (
    (open &&
      (planState.state === 'done' ? planState.r.options : must(last, 'the sections draw a career on show').r.results)
        .length > 1) ||
    refFor()
  )
    box.appendChild(optionsEl());
  renderSimulation();
  markOwn();
}
