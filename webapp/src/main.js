// Legacy Loop Planner page: entry point. build.py bundles this and the modules it imports (esbuild, one IIFE) into
// dist/index.html, next to the data, images, wasm and worker it embeds.
import { $ } from './dom.js';
import { SAVED, ERA } from './settings.js';
import { EXAMPLE, PRESET, loadData } from './data.js';
import { state, ui, writeSaved } from './state.js';
import { save, doSearch } from './search.js';
import { initRoster, renderRoster } from './roster.js';
import { initTargets, renderTargets } from './targets.js';
import { initCards, renderCards } from './cards.js';
import { renderDeck } from './results.js';
import { initPopover } from './popover.js';
import { initIO } from './io.js';

// ---------- game data ----------
$('era-global').setAttribute('aria-pressed', String(ERA === 'global')); $('era-gm').setAttribute('aria-pressed', String(ERA === 'gm'));
const setEra = (e) => { if (e === ERA) return; SAVED.era = e; writeSaved(); location.reload(); };
$('era-global').onclick = () => setEra('global'); $('era-gm').onclick = () => setEra('gm');
// everything below needs the era's data (inline for Global, fetched for the GM era)
loadData().then(start, (e) => { const p = document.createElement('p'); p.className = 'err'; p.textContent = e.message; $('results').replaceChildren(p); });
function start() {
  $('x-rcards').checked = state.rcards; $('x-rcards').onchange = (e) => { state.rcards = e.target.checked; save(); renderCards(); };
  $('x-rand').value = state.rand; $('x-rand').onchange = (e) => { state.rand = e.target.value; save(); };

  initRoster(); initCards(); initTargets(); initIO(); initPopover();
  for (const id of ['why-dlg', 'sched-dlg']) $(id).addEventListener('click', (e) => { if (e.target === $(id)) $(id).close(); });   // backdrop click closes

  renderRoster(); renderTargets();   // renderTargets draws the card picker too (it lists the cards that give a target)
  if (ERA === 'global') { ui.lastRender = () => renderDeck(EXAMPLE, { example: true, targets: PRESET, rank: '0.50,0.45,0.05' }); ui.lastRender(); ui.lastResult = { kind: 'deck', r: EXAMPLE, meta: { example: true } }; }
  else $('results').innerHTML = '';
  setTimeout(doSearch, 0);   // the first search, with the saved settings (also warms the engine)
}
