// ---------- target skills ----------
import { $ } from './dom.js';
import { CAT, SK } from './data.js';
import { initPresets } from './presets.js';
import { state, goldGiven } from './state.js';
import { skillIcon } from './images.js';
import { save } from './save.js';
import { renderCards } from './cards.js';
const namesOf = (group) => SK.get(group) || { w: `Skill ${group}`, gd: null };
// a target as the game's skill tile: icon and name; a gold the scenario guarantees shows as that gold (It's On! for
// Ramp Up), in gold (the editor below adds its remove button; the Presets view shows the tiles alone)
export function tileEl(group) {
  const names = namesOf(group);
  const tile = document.createElement('div'); tile.className = 'tgt'; tile.dataset.g = goldGiven(group) ? '2' : '0';
  const name = document.createElement('span'); name.className = 'name'; tile.appendChild(name);
  const gold = goldGiven(group) && !!names.gd;
  if (gold) tile.classList.add('gold');
  const icon = skillIcon(group, gold); if (icon) name.appendChild(icon);
  name.append(gold ? names.gd : names.w); if (gold) name.title = `Gold of ${names.w}, guaranteed by the scenario`;   // a plain target's name is on the tile: no hover
  return tile;
}
// one compact tile per target, two per row: icon and name (white → gold when the scenario guarantees the gold),
// remove. focus: after a removal, the row index whose remove button gets the focus (the next row's, else the last
// row's, else the add box), so the keyboard doesn't fall back to the page top
export function renderTargets(focus = null) {
  const count = document.getElementById('o-skills-n'); if (count) count.textContent = String(state.targets.length);   // the setup panel's Target skills button
  const box = $('targets'); box.textContent = '';
  state.targets.forEach((t, i) => {
    const names = namesOf(t.group);
    const tile = tileEl(t.group); tile.id = `g-${t.group}`;
    const remove = document.createElement('button'); remove.className = 'x'; remove.type = 'button'; remove.textContent = '×'; tile.appendChild(remove);
    remove.setAttribute('aria-label', `Remove ${names.w}`); remove.title = `Remove ${names.w}`;
    remove.onclick = () => { state.targets.splice(i, 1); save(); renderTargets(i); };
    box.appendChild(tile);
  });
  if (!state.targets.length) box.innerHTML = '<p class="hint">No target skills yet. Add one above.</p>';
  if (focus !== null) { const removes = box.querySelectorAll('.x'); (removes[Math.min(focus, removes.length - 1)] || $('t-add')).focus(); }
  renderCards();
}
export function initTargets() {
  const addIn = $('t-add'), suggestions = $('t-sugg');
  const SHOWN_MAX = 30;   // suggestions listed at most
  const items = () => [...suggestions.querySelectorAll('button')];
  const add = (group) => { state.targets.push({ group }); addIn.value = ''; suggestions.hidden = true; save(); renderTargets(); addIn.focus(); };
  const note = (text) => { const p = document.createElement('p'); p.className = 'snote'; p.textContent = text; suggestions.appendChild(p); };
  addIn.oninput = () => {
    const filter = addIn.value.trim().toLowerCase(); suggestions.textContent = '';
    if (filter.length < 2) { suggestions.hidden = true; return; }
    const have = new Set(state.targets.map(t => t.group));
    const matches = (name) => !!name && name.toLowerCase().includes(filter);
    const match = CAT.filter(([, white, gold]) => matches(white) || matches(gold));
    const hits = match.filter(([group]) => !have.has(group)).slice(0, SHOWN_MAX);
    for (const [group, white, gold] of hits) {
      // matched on the gold's name only: show the gold with its white (the target is the white; its gold comes with it)
      const viaGold = matches(gold) && !matches(white);
      const b = document.createElement('button'); b.type = 'button'; b.textContent = white;
      if (viaGold) { const arrow = document.createElement('span'); arrow.className = 'tg'; arrow.textContent = ` → ${gold}`; b.appendChild(arrow); }
      const icon = skillIcon(group, viaGold); if (icon) b.prepend(icon);
      b.onclick = () => add(group);
      suggestions.appendChild(b);
    }
    for (const [, white] of match.filter(([group]) => have.has(group)).slice(0, 3)) note(`${white} is already a target`);
    if (!match.length) note(`No skill matches “${addIn.value.trim()}”`);
    suggestions.hidden = false;
  };
  // Enter adds the first match; the arrows walk the list (Up from the first goes back to the box); Escape closes it
  addIn.onkeydown = (e) => {
    if (e.key === 'Escape' && !suggestions.hidden) { e.preventDefault(); suggestions.hidden = true; }   // the list, not the dialog the editor sits in
    else if (e.key === 'Enter') { const b = items()[0]; if (b && !suggestions.hidden) { e.preventDefault(); b.click(); } }
    else if (e.key === 'ArrowDown' && !suggestions.hidden && items().length) { e.preventDefault(); items()[0].focus(); }
  };
  suggestions.addEventListener('keydown', (e) => {
    const bs = items(), i = bs.indexOf(document.activeElement);
    if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); suggestions.hidden = true; addIn.focus(); }
    else if (e.key === 'ArrowDown') { e.preventDefault(); if (i < bs.length - 1) bs[i + 1].focus(); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); (i > 0 ? bs[i - 1] : addIn).focus(); }
  });
  document.addEventListener('click', (e) => { if (!suggestions.contains(e.target) && e.target !== addIn) suggestions.hidden = true; });
  initPresets(renderTargets);
}
