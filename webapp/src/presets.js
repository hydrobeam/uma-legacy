// ---------- target skill presets: named sets of targets (the Target skills panel's Presets dialog) ----------
// settings.presets: [{name, groups: [skill group, ...]}], saved with the other shared settings (both pages, both data sets)
// and carried by Export. The 8/8 example comes preloaded (settings.ts DEFAULTS) and is a preset like any other.
// The list: per preset its name and count (a click opens its skills as the editor's tiles, with Rename), Load, Overwrite
// (with the targets now) and a trash can; under it, Save current skills opens a small popup for the name (offered as
// "Preset N+1", N the presets now). Starting from a preset is load, edit the targets, save under a new name.
import { $, $dialog, $input, el, backdropCloses } from './dom.js';
import { PRESET, settings, setPresets, setTargets } from './settings.ts';
import { tileEl } from './targets.js';
import { must } from './must.ts';

const NAME_MAX = 40;
// the same skills, in any order
const sameSkills = (a, b) =>
  a.length === b.length && [...a].sort((x, y) => x - y).join() === [...b].sort((x, y) => x - y).join();
const skillCount = (n) => `${n} skill${n === 1 ? '' : 's'}`;
// the game has no trash can among its UI sprites: one drawn in its icon style, in the ink colour, the slits in the
// button face's colour (style.css .ptrash .slit)
const TRASH =
  '<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path fill="currentColor" d="M9.2 2.5h5.6l.9 1.9H20a1 1 0 0 1 1 1V7H3V5.4a1 1 0 0 1 1-1h4.3z"/>' +
  '<path fill="currentColor" d="M4.8 8.5h14.4l-1.1 12a2 2 0 0 1-2 1.8H7.9a2 2 0 0 1-2-1.8z"/><path class="slit" d="M9.6 11.5v7.4M14.4 11.5v7.4" stroke-width="1.8" stroke-linecap="round"/></svg>';
let rerender = () => {
  // replaced by initPresets
};
// an edit of the presets saves them without searching again; loading one changes the targets (a search)
// (a preset is named by its place in the list: the list is drawn again after every edit, and the dialogs over it are
// modal, so a place stays that preset's while its row or its view is open)

/** @param {readonly number[]} groups */
function apply(groups) {
  setTargets(groups.map((group) => ({ group })));
  rerender();
}
export const loadExample = () => apply(PRESET); // the planner's "no targets" status offers the default eight directly

// imported presets join the player's (an import never deletes one): an identical one is skipped, a different one under
// a taken name comes in as "name (imported)"
/**
 * @typedef {import('./settings.ts').Preset} Preset
 * @param {import('./settings.ts').SettingsView['presets']|null|undefined} mine @param {Preset[]} theirs @returns {Preset[]}
 */
export function mergePresets(mine, theirs) {
  const out = (mine || []).map((p) => ({ name: p.name, groups: [...p.groups] }));
  for (const p of theirs) {
    const name = String(p.name).trim().slice(0, NAME_MAX),
      have = out.find((q) => q.name === name);
    if (have && sameSkills(have.groups, p.groups)) continue;
    // the name cut so the suffix keeps it within NAME_MAX (" (imported)" is 11 characters, " (imported N)" 15 for N up to 999)
    let unique = have ? `${name.slice(0, NAME_MAX - 11)} (imported)` : name;
    for (let i = 2; out.some((q) => q.name === unique); i++) unique = `${name.slice(0, NAME_MAX - 15)} (imported ${i})`;
    out.push({ name: unique, groups: [...p.groups] });
  }
  return out;
}

/** @type {(name: string, except?: number) => boolean} except: the place of a preset whose own name doesn't count */
const nameTaken = (name, except = -1) => settings.value.presets.some((p, at) => at !== except && p.name === name);
// the name offered: "Preset N+1", N the presets now (counting on past a name already taken)
function suggestName() {
  let i = settings.value.presets.length + 1;
  while (nameTaken(`Preset ${i}`)) i++;
  return `Preset ${i}`;
}

function renderList(msg = '') {
  const box = $('pr-list');
  box.textContent = '';
  const current = settings.value.targets.map((t) => t.group);
  if (!settings.value.presets.length) box.append(el('p', 'hint', 'No presets yet.'));
  for (const [at, p] of settings.value.presets.entries()) {
    const row = el('div', 'prow');
    row.setAttribute('role', 'group');
    row.setAttribute('aria-label', p.name);
    // the name and count, the whole left of the row: a click shows the preset's skills
    const view = el('button', 'pview');
    view.type = 'button';
    view.setAttribute('aria-haspopup', 'dialog');
    view.append(el('span', 'pname', p.name), el('span', 'pmeta', skillCount(p.groups.length)));
    if (sameSkills(p.groups, current)) {
      row.classList.add('cur');
      view.append(el('span', 'pcur', 'current'));
    }
    view.onclick = () => openView(at);
    const buttons = el('div', 'pbtns'),
      button = (text) => {
        const b = el('button', 'btn', text);
        b.type = 'button';
        buttons.append(b);
        return b;
      };
    button('Load').onclick = () => {
      apply(p.groups);
      $dialog('preset-dlg').close();
    };
    const overwrite = button('Overwrite');
    overwrite.disabled = !current.length; // as Save: a preset is never emptied
    overwrite.title = current.length ? '' : 'Add target skills first';
    overwrite.onclick = () => {
      setPresets(settings.value.presets.map((q, j) => (j === at ? { ...q, groups: [...current] } : q)));
      renderList(`Overwrote “${p.name}” with your ${skillCount(current.length)}.`);
    };
    const trash = button('');
    trash.classList.add('ptrash');
    trash.innerHTML = TRASH;
    trash.setAttribute('aria-label', 'Delete');
    trash.title = 'Delete';
    trash.onclick = () => {
      setPresets(settings.value.presets.filter((_, j) => j !== at));
      renderList(`Deleted “${p.name}”.`);
    };
    row.append(view, buttons);
    box.append(row);
  }
  $('pr-msg').textContent = msg;
  /** @type {HTMLButtonElement} */ ($('pr-save')).disabled = !settings.value.targets.length;
  $('pr-save').title = settings.value.targets.length ? '' : 'Add target skills first';
}

// a preset's skills as the editor's tiles; Rename opens its name for editing, Close
/** @type {number | null} the place of the preset on view */
let viewing = null;
/** @param {number} at */
function openView(at) {
  const p = settings.value.presets[at];
  viewing = at;
  $('prv-title').textContent = p.name;
  $('prv-msg').textContent = '';
  const box = $('prv-tiles');
  box.textContent = '';
  for (const g of p.groups) box.append(tileEl(g));
  if (!p.groups.length) box.append(el('p', 'hint', 'No skills.'));
  renameMode(false);
  $dialog('pr-view').showModal();
}
function renameMode(on) {
  $('prv-ren').hidden = !on;
  $('prv-rename').textContent = on ? 'Save name' : 'Rename';
  if (on) {
    const input = $input('prv-name');
    input.value = settings.value.presets[must(viewing, 'Rename is in the open preset')].name;
    input.focus();
    input.select();
  }
}
function commitRename() {
  const at = must(viewing, 'Save name is in the open preset'),
    preset = settings.value.presets[at];
  const name = $input('prv-name').value.trim();
  if (!name || name === preset.name) {
    renameMode(false);
    return;
  }
  if (nameTaken(name, at)) {
    $('prv-msg').textContent = `“${name}” is taken: names are unique.`;
    return;
  }
  const old = preset.name;
  setPresets(settings.value.presets.map((q, j) => (j === at ? { ...q, name } : q)));
  $('prv-title').textContent = name;
  $('prv-msg').textContent = `Renamed “${old}” to “${name}”.`;
  renameMode(false);
  renderList();
}

function saveNew() {
  const name = $input('prn-name').value.trim().slice(0, NAME_MAX);
  if (!name) {
    $('prn-msg').textContent = 'Give it a name.';
    return;
  }
  if (nameTaken(name)) {
    $('prn-msg').textContent = `“${name}” is taken: pick another name, or use that preset's Overwrite.`;
    return;
  }
  setPresets([...settings.value.presets, { name, groups: settings.value.targets.map((t) => t.group) }]);
  $dialog('pr-new').close();
  renderList(`Saved “${name}”.`);
}

export function initPresets(render) {
  rerender = render;
  const list = $dialog('preset-dlg');
  if (!list) return;
  $('t-preset').onclick = () => {
    renderList();
    list.showModal();
  };
  for (const dialog of [list, $dialog('pr-new'), $dialog('pr-view')]) {
    backdropCloses(dialog);
    dialog.querySelectorAll('.dclose').forEach((done) => {
      done.onclick = () => dialog.close();
    });
  }
  $('pr-save').onclick = () => {
    if (!settings.value.targets.length) return;
    $input('prn-name').value = suggestName();
    $('prn-msg').textContent = '';
    $dialog('pr-new').showModal();
    $input('prn-name').select();
  };
  $('prn-save').onclick = saveNew;
  $input('prn-name').onkeydown = (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      saveNew();
    }
  };
  $('prv-rename').onclick = () => {
    if ($('prv-ren').hidden) renameMode(true);
    else commitRename();
  };
  $input('prv-name').onkeydown = (e) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      commitRename();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      renameMode(false);
    } // the rename, not the popup
  };
}
