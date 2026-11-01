// ---------- the status line heading the results, and its copy in the phone's floating button ----------
// While a search runs: its time and a Stop button; otherwise the status (state.ts loopView). Whenever the results on
// screen are not those of the current settings (an error, an empty state, a stopped or running search) they stay,
// dimmed, instead of being wiped. Drawn from state.ts's records by main.js's effect on them (and by main.js before the
// first search), read with .peek(): what an effect calls subscribes it to nothing more.
import { $ } from '../dom.js';
import { runnerHTML } from '../images.js';
import { must } from '../must.ts';
import { headline, loopRun, loopView, resultsStale } from './state.ts';

// the button beside the status line, rebuilt only when it changes (keeps focus)
let actKey = null;
function setAct(key, label, onClick) {
  if (key === actKey) return;
  actKey = key;
  const box = $('r-act');
  box.textContent = '';
  if (!key) return;
  const b = document.createElement('button');
  b.type = 'button';
  b.className = 'btn';
  b.textContent = label;
  b.onclick = onClick;
  box.appendChild(b);
}
/** @param {() => void} stop the Stop button's action (run.ts stopSearch) */
export function paintRunNote(stop) {
  const run = loopRun.peek(),
    status = loopView.peek().status;
  const note = $('run-note'),
    results = $('results'),
    hasResults = !!results.querySelector('.loop');
  note.classList.toggle('busy', run.searching);
  $('rhead').dataset.kind = run.searching ? 'busy' : status.kind;
  results.setAttribute('aria-busy', String(run.searching));
  results.classList.toggle('stale', resultsStale.peek() && !run.searching);
  let pill;
  if (run.searching) {
    const sec = Math.floor((run.now - run.startedAt) / 1000),
      time = sec >= 1 ? ` ${sec} s` : '';
    // the seconds are hidden from screen readers: the live region would read them out every second
    const what = run.phase === 'check' ? 'Checking the exact odds…' : 'Searching…';
    // one line beside the spinner: the exact check's caveat is its tooltip (and read to screen readers)
    const caveat =
      run.phase === 'check'
        ? '<span class="vh"> The result below may still change.</span>'
        : sec >= 1 && hasResults
          ? '. The results below are from before your change.'
          : '';
    note.innerHTML = `${runnerHTML()}<span>${what}<span aria-hidden="true">${time}</span>${caveat}</span>`;
    note.title =
      run.phase === 'check' ? 'The result below is ranked by quick odds; the exact check can still reorder it' : '';
    setAct('stop', 'Stop', stop);
    pill = what + time;
  } else {
    note.textContent = status.text;
    note.title = '';
    const act = status.act;
    if (act)
      setAct(
        act.click || act.label,
        act.label,
        act.run || (() => $(must(act.click, 'an act without run names the button it presses')).click()),
      );
    else setAct(null);
    const answered = status.kind === 'done' || status.kind === 'example';
    const top = headline.peek();
    pill = answered
      ? top
        ? `${top} · see result ↑`
        : status.text
      : status.kind === 'loading'
        ? status.text
        : 'Not searched · see why ↑';
  }
  const pillButton = $('run-pill');
  pillButton.hidden = !note.textContent;
  pillButton.textContent = pill;
}
