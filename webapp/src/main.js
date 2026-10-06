// Legacy Loop Planner page: entry point. build.py bundles this and the modules it imports (esbuild, one IIFE) into
// dist/index.html, next to the data, images, wasm and worker it embeds.
import { $ } from './dom.js';
import { SAVED, ERA } from './settings.js';
import { PRESET, loadData } from './data.js';
import { state, ui, writeSaved } from './state.js';
import { save, doSearch, firstResult, paintRunNote } from './search.js';
import { initRoster, renderRoster } from './roster.js';
import { initTargets, renderTargets } from './targets.js';
import { initScenario, eraScenario } from './scenario.js';
import { initCards, renderCards } from './cards.js';
import { renderDeck } from './results.js';
import { initPopover } from './popover.js';
import { initIO } from './io.js';

// ---------- results column (desktop): sticky by its top when it fits the window, by its bottom when it is taller ----------
{ const rcol = document.querySelector('.rcol');
  if (rcol) { const fit = () => rcol.style.setProperty('--rtop', Math.min(12, innerHeight - rcol.offsetHeight - 12) + 'px');
    new ResizeObserver(fit).observe(rcol); addEventListener('resize', fit); } }
// ---------- game data ----------
$('era-global').setAttribute('aria-pressed', String(ERA === 'global')); $('era-gm').setAttribute('aria-pressed', String(ERA === 'gm'));
const setEra = (e) => { if (e === ERA) return; eraScenario(e); SAVED.era = e; writeSaved(); location.reload(); };
$('era-global').onclick = () => setEra('global'); $('era-gm').onclick = () => setEra('gm');
// everything below needs the era's data (inline for Global, fetched for the GM era)
if (ERA === 'gm') { ui.status = { kind: 'loading', text: 'Loading the JP Grand Masters-era data…' }; paintRunNote(); }   // fetched: about a second
loadData().then(start, (e) => { ui.status = { kind: 'error', text: e.message }; paintRunNote(); });
function start() {
  $('x-auto').checked = state.auto !== false; $('x-auto').onchange = (e) => { state.auto = e.target.checked; writeSaved(); if (state.auto && ui.status.kind === 'pending') save(); };   // the switch changes no result: switching it on searches only changes made while it was off
  initScenario();   // the scenario picker (sets the training-hint rate; Unity Cup's golds follow it)

  initRoster(); initCards(); initTargets(); initIO(); initPopover();
  // phone: the floating status button shows while the results are out of view, and takes you back up to them
  if (window.IntersectionObserver) new IntersectionObserver((es) => document.documentElement.classList.toggle('rin', es[0].isIntersecting)).observe($('rcol'));
  $('run-pill').onclick = () => { const c = $('rcol'); c.scrollIntoView({ block: 'start', behavior: matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth' }); c.focus({ preventScroll: true }); };
  // the roster and card boxes: .more while there is more to scroll to below (their bottom edge fades)
  for (const id of ['roster', 'cards']) { const el = $(id), upd = () => el.classList.toggle('more', el.scrollTop + el.clientHeight < el.scrollHeight - 4);
    el.addEventListener('scroll', upd, { passive: true }); new MutationObserver(upd).observe(el, { childList: true }); addEventListener('resize', upd); }
  for (const id of ['why-dlg', 'sched-dlg']) $(id).addEventListener('click', (e) => { if (e.target === $(id)) $(id).close(); });   // backdrop click closes

  renderRoster(); renderTargets();   // renderTargets draws the card picker too (it lists the cards that give a target)
  // before the first search finishes: the remembered result of these settings, or the example for the defaults, or nothing
  // (showing the default example under other settings flashed a result that wasn't theirs)
  const first = firstResult();
  if (first) { ui.lastRender = () => renderDeck(first.r, { targets: first.example ? PRESET : first.targets });
    ui.lastRender(); ui.lastResult = { kind: 'deck', r: first.r, meta: first.example ? { example: true } : {} };
    ui.status = first.example ? { kind: 'example', text: 'Showing an example: Unity Cup skills, everything owned' } : { kind: 'done', text: 'Your last result for these settings' }; }
  setTimeout(doSearch, 0);   // the first search, with the saved settings (also warms the engine)
}
