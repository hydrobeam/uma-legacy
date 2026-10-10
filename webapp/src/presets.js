// ---------- target skill presets: named sets of targets (the Target skills panel's Presets dialog) ----------
// state.presets: [{name, groups: [skill group, ...]}], saved with the other shared settings (both pages, both data sets)
// and carried by Export. The 8/8 example comes preloaded (state.js DEFAULTS) and is a preset like any other. A row
// shows the name and count; a click on it opens its skills as the editor's tiles. Starting from a preset is load, edit
// the targets, save under a new name.
import { $, el, backdropCloses } from './dom.js';
import { PRESET } from './data.js';
import { state } from './state.js';
import { save } from './save.js';
import { tileEl } from './targets.js';

const NAME_MAX = 40;
const same = (a, b) => a.length === b.length && [...a].sort((x, y) => x - y).join() === [...b].sort((x, y) => x - y).join();
const count = (n) => `${n} skill${n === 1 ? '' : 's'}`;
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

// a button that acts on its second click within 4 s (Update and Delete lose what the preset held)
function twoStep(b, ask, run) {
  const label = b.textContent; let t = 0;
  b.onclick = () => {
    if (!b.classList.contains('armed')) { b.classList.add('armed'); b.textContent = ask; t = setTimeout(() => { b.classList.remove('armed'); b.textContent = label; }, 4000); return; }
    clearTimeout(t); run();
  };
}

const nameTaken = (name, except = null) => state.presets.some(p => p !== except && p.name === name);
function freeName() { let i = state.presets.length + 1; while (nameTaken(`Preset ${i}`)) i++; return `Preset ${i}`; }

function renderList(msg = '') {
  const box = $('pr-list'); box.textContent = '';
  const cur = state.targets.map(t => t.group);
  if (!state.presets.length) box.append(el('p', 'hint', 'No presets yet. Save your targets below.'));
  for (const p of state.presets) {
    const row = el('div', 'prow'); row.setAttribute('role', 'group'); row.setAttribute('aria-label', p.name);
    const head = el('div', 'phead');
    // the name and count: a click shows the preset's skills
    const view = el('button', 'pview'); view.type = 'button'; view.setAttribute('aria-haspopup', 'dialog'); view.title = 'Show its skills';
    view.append(el('span', 'pname', p.name), el('span', 'pmeta', count(p.groups.length)));
    const pen = el('button', 'pren-b', '✎'); pen.type = 'button'; pen.title = 'Rename'; pen.setAttribute('aria-label', 'Rename');
    view.onclick = () => openView(p); pen.onclick = () => rename(p, view);
    head.append(view, pen);
    if (same(p.groups, cur)) { row.classList.add('cur'); head.append(el('span', 'pcur', 'current')); }
    const btns = el('div', 'pbtns'), btn = (t) => { const b = el('button', 'btn', t); b.type = 'button'; btns.append(b); return b; };
    btn('Load').onclick = () => { apply(p.groups); $('preset-dlg').close(); };
    twoStep(btn('Update'), 'Replace?', () => { p.groups = [...cur]; save(); renderList(`Updated “${p.name}” with your ${count(cur.length)}.`); });
    twoStep(btn('Delete'), 'Delete?', () => { state.presets = state.presets.filter(q => q !== p); save(); renderList(`Deleted “${p.name}”.`); });
    row.append(head, btns); box.append(row);
  }
  $('pr-msg').textContent = msg;
  $('pr-save').disabled = !state.targets.length;
  $('pr-name').placeholder = state.targets.length ? freeName() : 'Add targets first';
}

// a preset's skills as the editor's tiles, with Load
function openView(p) {
  const d = $('pr-view');
  $('prv-title').textContent = p.name;
  const box = $('prv-tiles'); box.textContent = '';
  for (const g of p.groups) box.append(tileEl(g));
  if (!p.groups.length) box.append(el('p', 'hint', 'No skills.'));
  $('prv-load').onclick = () => { apply(p.groups); d.close(); $('preset-dlg').close(); };
  d.showModal();
}

function rename(p, node) {
  const inp = el('input'); inp.type = 'text'; inp.className = 'pren'; inp.value = p.name; inp.maxLength = NAME_MAX; inp.setAttribute('aria-label', 'New name for ' + p.name);
  node.replaceWith(inp); inp.focus(); inp.select();
  let done = false;
  const commit = (keep) => {
    if (done) return; done = true;
    const v = inp.value.trim();
    if (keep && v && v !== p.name) {
      if (nameTaken(v, p)) { renderList(`“${v}” is taken: names are unique.`); return; }
      const old = p.name; p.name = v; save(); renderList(`Renamed “${old}” to “${v}”.`); return;
    }
    renderList();
  };
  inp.onkeydown = (e) => {
    if (e.key === 'Enter') { e.preventDefault(); commit(true); }
    else if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); commit(false); }   // the rename, not the dialog
  };
  inp.onblur = () => commit(true);
}

export function initPresets(render) {
  rerender = render;
  const d = $('preset-dlg'); if (!d) return;
  $('t-preset').onclick = () => { $('pr-name').value = ''; renderList(); d.showModal(); };
  for (const x of [d, $('pr-view')]) { backdropCloses(x); x.querySelectorAll('.dclose').forEach(c => { c.onclick = () => x.close(); }); }
  const saveNew = () => {
    if (!state.targets.length) return;
    const name = ($('pr-name').value.trim() || $('pr-name').placeholder).slice(0, NAME_MAX);
    if (nameTaken(name)) { $('pr-msg').textContent = `“${name}” is taken: pick another name, or use that preset's Update.`; return; }
    state.presets = [...state.presets, { name, groups: state.targets.map(t => t.group) }]; save();
    $('pr-name').value = ''; renderList(`Saved “${name}”.`);
  };
  $('pr-save').onclick = saveNew;
  $('pr-name').onkeydown = (e) => { if (e.key === 'Enter') { e.preventDefault(); saveNew(); } };
}
