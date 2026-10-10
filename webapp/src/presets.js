// ---------- target skill presets: named sets of targets (the Target skills panel's Presets dialog) ----------
// state.presets: [{name, groups: [skill group, ...]}], saved with the other shared settings (both pages, both data sets)
// and carried by Export. The 8/8 example comes preloaded (state.js DEFAULTS) and is a preset like any other.
// The list: per preset its name and count (a click opens its skills as the editor's tiles, with Rename), Load, Overwrite
// (with the targets now) and a trash can (a second click deletes); under it, Save current skills opens a small popup
// for the name. Starting from a preset is load, edit the targets, save under a new name.
import { $, el, backdropCloses } from './dom.js';
import { PRESET } from './data.js';
import { state } from './state.js';
import { save } from './save.js';
import { tileEl } from './targets.js';

const NAME_MAX = 40, EXAMPLE_NAME = '8/8 example';
const same = (a, b) => a.length === b.length && [...a].sort((x, y) => x - y).join() === [...b].sort((x, y) => x - y).join();
const count = (n) => `${n} skill${n === 1 ? '' : 's'}`;
// the game has no trash can among its UI sprites: one drawn in its icon style, in the ink colour, the slits in the face's
const TRASH = '<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path fill="currentColor" d="M9.2 2.5h5.6l.9 1.9H20a1 1 0 0 1 1 1V7H3V5.4a1 1 0 0 1 1-1h4.3z"/>'
  + '<path fill="currentColor" d="M4.8 8.5h14.4l-1.1 12a2 2 0 0 1-2 1.8H7.9a2 2 0 0 1-2-1.8z"/><path class="slit" d="M9.6 11.5v7.4M14.4 11.5v7.4" stroke-width="1.8" stroke-linecap="round"/></svg>';
let rerender = () => {};

function apply(groups) { state.targets = groups.map(group => ({ group })); save(); rerender(); }
export const loadExample = () => apply(PRESET);   // the planner's "no targets" status offers the default eight directly

// imported presets join the player's (an import never deletes one): an identical one is skipped, a different one under
// a taken name comes in as "name (imported)"
export function mergePresets(mine, theirs) {
  const out = (mine || []).map(p => ({ name: p.name, groups: [...p.groups] }));
  for (const p of theirs) {
    const name = String(p.name).trim().slice(0, NAME_MAX), have = out.find(q => q.name === name);
    if (have && same(have.groups, p.groups)) continue;
    let n = have ? `${name.slice(0, NAME_MAX - 11)} (imported)` : name;
    for (let i = 2; out.some(q => q.name === n); i++) n = `${name.slice(0, NAME_MAX - 15)} (imported ${i})`;
    out.push({ name: n, groups: [...p.groups] });
  }
  return out;
}

const nameTaken = (name, except = null) => state.presets.some(p => p !== except && p.name === name);
// the name offered for the targets now: the default eight's own name when they are the default eight, else "Preset N"
function suggestName() {
  const cur = state.targets.map(t => t.group);
  if (same(cur, PRESET) && !nameTaken(EXAMPLE_NAME)) return EXAMPLE_NAME;
  let i = state.presets.length + 1; while (nameTaken(`Preset ${i}`)) i++; return `Preset ${i}`;
}

function renderList(msg = '') {
  const box = $('pr-list'); box.textContent = '';
  const cur = state.targets.map(t => t.group);
  if (!state.presets.length) box.append(el('p', 'hint', 'No presets yet.'));
  for (const p of state.presets) {
    const row = el('div', 'prow'); row.setAttribute('role', 'group'); row.setAttribute('aria-label', p.name);
    // the name and count, the whole left of the row: a click shows the preset's skills
    const view = el('button', 'pview'); view.type = 'button'; view.setAttribute('aria-haspopup', 'dialog');
    view.append(el('span', 'pname', p.name), el('span', 'pmeta', count(p.groups.length)));
    if (same(p.groups, cur)) { row.classList.add('cur'); view.append(el('span', 'pcur', 'current')); }
    view.onclick = () => openView(p);
    const btns = el('div', 'pbtns'), btn = (t) => { const b = el('button', 'btn', t); b.type = 'button'; btns.append(b); return b; };
    btn('Load').onclick = () => { apply(p.groups); $('preset-dlg').close(); };
    btn('Overwrite').onclick = () => { p.groups = [...cur]; save(); renderList(`Overwrote “${p.name}” with your ${count(cur.length)}.`); };
    // the trash can deletes on a second click within 4 s (the first turns it into "Delete?")
    const del = btn(''); del.classList.add('ptrash'); del.innerHTML = TRASH; del.setAttribute('aria-label', 'Delete'); del.title = 'Delete';
    let t = 0;
    del.onclick = () => {
      if (!del.classList.contains('armed')) { del.classList.add('armed'); del.append(el('span', '', 'Delete?')); del.setAttribute('aria-label', 'Delete? Click again to delete');
        t = setTimeout(() => { del.classList.remove('armed'); del.lastChild.remove(); del.setAttribute('aria-label', 'Delete'); }, 4000); return; }
      clearTimeout(t); state.presets = state.presets.filter(q => q !== p); save(); renderList(`Deleted “${p.name}”.`);
    };
    row.append(view, btns); box.append(row);
  }
  $('pr-msg').textContent = msg;
  $('pr-save').disabled = !state.targets.length;
  $('pr-save').title = state.targets.length ? '' : 'Add target skills first';
}

// a preset's skills as the editor's tiles; Rename opens its name for editing, Close
let viewing = null;
function openView(p) {
  viewing = p; $('prv-title').textContent = p.name; $('prv-msg').textContent = '';
  const box = $('prv-tiles'); box.textContent = '';
  for (const g of p.groups) box.append(tileEl(g));
  if (!p.groups.length) box.append(el('p', 'hint', 'No skills.'));
  renameMode(false);
  $('pr-view').showModal();
}
function renameMode(on) {
  $('prv-ren').hidden = !on; $('prv-rename').textContent = on ? 'Save name' : 'Rename';
  if (on) { const i = $('prv-name'); i.value = viewing.name; i.focus(); i.select(); }
}
function commitRename() {
  const v = $('prv-name').value.trim();
  if (!v || v === viewing.name) { renameMode(false); return; }
  if (nameTaken(v, viewing)) { $('prv-msg').textContent = `“${v}” is taken: names are unique.`; return; }
  const old = viewing.name; viewing.name = v; save();
  $('prv-title').textContent = v; $('prv-msg').textContent = `Renamed “${old}” to “${v}”.`; renameMode(false); renderList();
}

function saveNew() {
  const name = $('prn-name').value.trim().slice(0, NAME_MAX);
  if (!name) { $('prn-msg').textContent = 'Give it a name.'; return; }
  if (nameTaken(name)) { $('prn-msg').textContent = `“${name}” is taken: pick another name, or use that preset's Overwrite.`; return; }
  state.presets = [...state.presets, { name, groups: state.targets.map(t => t.group) }]; save();
  $('pr-new').close(); renderList(`Saved “${name}”.`);
}

export function initPresets(render) {
  rerender = render;
  const d = $('preset-dlg'); if (!d) return;
  $('t-preset').onclick = () => { renderList(); d.showModal(); };
  for (const x of [d, $('pr-new'), $('pr-view')]) { backdropCloses(x); x.querySelectorAll('.dclose').forEach(c => { c.onclick = () => x.close(); }); }
  $('pr-save').onclick = () => { if (!state.targets.length) return;
    $('prn-name').value = suggestName(); $('prn-msg').textContent = ''; $('pr-new').showModal(); $('prn-name').select(); };
  $('prn-save').onclick = saveNew;
  $('prn-name').onkeydown = (e) => { if (e.key === 'Enter') { e.preventDefault(); saveNew(); } };
  $('prv-rename').onclick = () => { if ($('prv-ren').hidden) renameMode(true); else commitRename(); };
  $('prv-name').onkeydown = (e) => {
    if (e.key === 'Enter') { e.preventDefault(); commitRename(); }
    else if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); renameMode(false); }   // the rename, not the popup
  };
}
