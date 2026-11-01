// The Borrowed parent choice of the career setup.
import { $ } from '../../dom.js';
import { settings, setBorrowPar } from '../../settings.ts';
import { must } from '../../must.ts';
import { borrowChoice } from '../job.js';

// Borrowed parent: three choices, saved with the other settings (a change runs the planner again: initSettings)
function paintBorrow() {
  for (const b of $('bp').querySelectorAll('button'))
    b.setAttribute('aria-pressed', String(b.dataset.v === borrowChoice(settings.value)));
}
export function initBorrow() {
  for (const b of $('bp').querySelectorAll('button'))
    b.onclick = () => {
      if (b.dataset.v === borrowChoice(settings.value)) return;
      setBorrowPar(must(b.dataset.v, 'each Borrowed parent choice names its value'));
      paintBorrow();
    };
  paintBorrow();
}
