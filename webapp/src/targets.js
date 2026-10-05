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
// What is guaranteed outside the deck follows the scenario alone (Unity Cup's golds); everything else the search
// works out itself (outfits, decks, events), so the player doesn't set it. Fixes saved hand-set values too.
export function scenGuarantees() { const g = SCEN_GOLDS[state.scen || 'unity'] || []; for (const t of state.targets) t.guaranteed = g.includes(t.group) ? 2 : 0; }
// one compact tile per target, two per row: icon and name (white → gold when the scenario guarantees the gold),
// remove. focus: after a removal, the row index whose remove button gets the focus (the next row's, else the last
// row's, else the add box), so the keyboard doesn't fall back to the page top
export function renderTargets(focus = null) {
  const box = $('targets'); box.textContent = '';
  scenGuarantees();
  state.targets.forEach((t, i) => {
    const s = SK.get(t.group) || { w: 'Skill ' + t.group, gd: null };
    const el = document.createElement('div'); el.className = 'tgt'; el.id = 'g-' + t.group; el.dataset.g = String(t.guaranteed || 0);
    el.innerHTML = `<span class="name"></span><button class="x" type="button">×</button>`;
    const nm = el.querySelector('.name');
    const ic = skillIcon(t.group, t.guaranteed === 2); if (ic) nm.appendChild(ic);
    nm.append(s.w); if (t.guaranteed === 2 && s.gd) { const g = document.createElement('span'); g.className = 'tg'; g.textContent = ' → ' + s.gd; g.title = 'Guaranteed by the scenario'; nm.appendChild(g); }
    nm.title = s.w + (t.guaranteed === 2 && s.gd ? ' → ' + s.gd + ' (guaranteed by the scenario)' : '');
    const x = el.querySelector('.x'); x.setAttribute('aria-label', 'Remove ' + s.w); x.title = 'Remove ' + s.w;
    x.onclick = () => { state.targets.splice(i, 1); save(); renderTargets(i); };
    box.appendChild(el);
  });
  if (!state.targets.length) box.innerHTML = '<p class="hint">No target skills yet. Add one below.</p>';
  if (focus !== null) { const xs = box.querySelectorAll('.x'); (xs[Math.min(focus, xs.length - 1)] || $('t-add')).focus(); }
  renderCards();
}
export function initTargets() {
  const addIn = $('t-add'), sugg = $('t-sugg');
  const items = () => [...sugg.querySelectorAll('button')];
  const add = (g) => { state.targets.push({ group: g, guaranteed: 0 }); addIn.value = ''; sugg.hidden = true; save(); renderTargets(); addIn.focus(); };
  const note = (text) => { const p = document.createElement('p'); p.className = 'snote'; p.textContent = text; sugg.appendChild(p); };
  addIn.oninput = () => {
    const f = addIn.value.trim().toLowerCase(); sugg.textContent = '';
    if (f.length < 2) { sugg.hidden = true; return; }
    const have = new Set(state.targets.map(t => t.group));
    const match = CAT.filter(([, w, gd]) => w.toLowerCase().includes(f) || (gd && gd.toLowerCase().includes(f)));
    const hits = match.filter(([g]) => !have.has(g)).slice(0, 30);
    for (const [g, w, gd] of hits) {
      // matched on the gold's name only: show the gold with its white (the target is the white; its gold comes with it)
      const viaGold = gd && gd.toLowerCase().includes(f) && !w.toLowerCase().includes(f);
      const b = document.createElement('button'); b.type = 'button'; b.textContent = w;
      if (viaGold) { const x = document.createElement('span'); x.className = 'tg'; x.textContent = ' → ' + gd; b.appendChild(x); }
      const ic = skillIcon(g, viaGold); if (ic) b.prepend(ic);
      b.onclick = () => add(g);
      sugg.appendChild(b);
    }
    for (const [g, w] of match.filter(([g]) => have.has(g)).slice(0, 3)) note(`${w} is already a target`);
    if (!match.length) note('No skill matches “' + addIn.value.trim() + '”');
    sugg.hidden = false;
  };
  // Enter adds the first match; the arrows walk the list (Up from the first goes back to the box); Escape closes it
  addIn.onkeydown = (e) => {
    if (e.key === 'Escape') sugg.hidden = true;
    else if (e.key === 'Enter') { const b = items()[0]; if (b && !sugg.hidden) { e.preventDefault(); b.click(); } }
    else if (e.key === 'ArrowDown' && !sugg.hidden && items().length) { e.preventDefault(); items()[0].focus(); }
  };
  sugg.addEventListener('keydown', (e) => {
    const bs = items(), i = bs.indexOf(document.activeElement);
    if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); sugg.hidden = true; addIn.focus(); }
    else if (e.key === 'ArrowDown') { e.preventDefault(); if (i < bs.length - 1) bs[i + 1].focus(); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); (i > 0 ? bs[i - 1] : addIn).focus(); }
  });
  document.addEventListener('click', (e) => { if (!sugg.contains(e.target) && e.target !== addIn) sugg.hidden = true; });
  $('t-preset').onclick = () => { state.targets = PRESET.map(t => ({ group: t.group, guaranteed: 0 })); scenarioGolds(null, state.scen); save(); renderTargets(); };
}
