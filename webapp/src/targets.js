// ---------- target skills, career-event extras ----------
import { $ } from './dom.js';
import { CAT, SK, PRESET, SCEN_GOLDS } from './data.js';
import { state } from './state.js';
import { skillIcon } from './images.js';
import { save } from './search.js';
import { renderCards } from './cards.js';
// Apply the scenario's guaranteed golds to the target list: targets the new scenario guarantees become guaranteed
// golds; golds only the old scenario guaranteed go back to none.
export function scenarioGolds(from, to) {
  const was = SCEN_GOLDS[from] || [], now = SCEN_GOLDS[to] || [];
  for (const t of state.targets) {
    if (now.includes(t.group)) t.guaranteed = 2;
    else if (was.includes(t.group) && t.guaranteed === 2) t.guaranteed = 0;
  }
}
export function renderTargets() {
  const box = $('targets'); box.textContent = '';
  state.targets.forEach((t, i) => {
    const s = SK.get(t.group) || { w: 'Skill ' + t.group, gd: null };
    const el = document.createElement('div'); el.className = 'tgt';
    el.innerHTML = `<div class="row between"><div><span class="name"></span></div><button class="x" aria-label="Remove">×</button></div>
      <div class="ctrl">
        <label>Guaranteed outside the deck<select data-k="guaranteed"><option value="0">None</option><option value="1">White</option>${s.gd ? '<option value="2">Gold</option>' : ''}</select></label>
      </div>`;
    const nm = el.querySelector('.name'); nm.textContent = s.w; const ic = skillIcon(t.group); if (ic) nm.prepend(ic);
    const sel = el.querySelector('select'); sel.value = String(t.guaranteed || 0); sel.id = 'g-' + t.group;
    sel.onchange = () => { t.guaranteed = +sel.value; save(); };
    el.querySelector('.x').onclick = () => { state.targets.splice(i, 1); save(); renderTargets(); };
    box.appendChild(el);
  });
  if (!state.targets.length) box.innerHTML = '<p class="hint">No target skills yet. Add one below.</p>';
  renderCards();
}
export function initTargets() {
  const addIn = $('t-add'), sugg = $('t-sugg');
  addIn.oninput = () => {
    const f = addIn.value.trim().toLowerCase(); sugg.textContent = '';
    if (f.length < 2) { sugg.hidden = true; return; }
    const have = new Set(state.targets.map(t => t.group));
    const hits = CAT.filter(([g, w, gd]) => !have.has(g) && (w.toLowerCase().includes(f) || (gd && gd.toLowerCase().includes(f)))).slice(0, 30);
    for (const [g, w] of hits) {
      const b = document.createElement('button'); b.textContent = w; const ic = skillIcon(g); if (ic) b.prepend(ic);
      b.onclick = () => { state.targets.push({ group: g, guaranteed: 0 }); addIn.value = ''; sugg.hidden = true; save(); renderTargets(); };
      sugg.appendChild(b);
    }
    sugg.hidden = !hits.length;
  };
  addIn.onkeydown = (e) => { if (e.key === 'Escape') sugg.hidden = true; if (e.key === 'ArrowDown' && sugg.firstChild) { e.preventDefault(); sugg.firstChild.focus(); } };
  document.addEventListener('click', (e) => { if (!sugg.contains(e.target) && e.target !== addIn) sugg.hidden = true; });
  $('t-preset').onclick = () => { state.targets = PRESET.map(t => ({ group: t.group, guaranteed: 0 })); scenarioGolds(null, state.scen); save(); renderTargets(); };
  $('x-zephyr').checked = state.zephyr; $('x-zephyr').onchange = (e) => { state.zephyr = e.target.checked; save(); };
}
