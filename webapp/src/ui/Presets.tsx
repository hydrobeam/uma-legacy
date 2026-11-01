// ---------- the Presets dialog (presets.js holds the edits): the list, Save current skills, a preset's view ----------
// The list: per preset its name and count (a click opens its skills as the editor's tiles, with Rename), Load, Overwrite
// (with the targets now) and a trash can; under it, Save current skills opens a small popup for the name. Each preset is
// named by its name, unique. The Target skills panel's Presets button calls `openPresets`.
import { batch, signal } from '@preact/signals';
import { useLayoutEffect, useRef, useState } from 'preact/hooks';
import { fields } from '../settings.ts';
import type { Preset } from '../settings.ts';
import type { DeepReadonly } from '../store.ts';
import {
  deletePreset,
  loadPreset,
  overwritePreset,
  renamePreset,
  saveCurrent,
  sameSkills,
  skillCount,
  suggestName,
} from '../presets.js';
import { must } from '../must.ts';
import { Dialog } from './Dialog.tsx';
import { TargetTile } from './TargetTile.tsx';

// the list dialog: open, and what its status line says (cleared each time it opens)
const listOpen = signal(false),
  listSaid = signal('');
export function openPresets() {
  batch(() => {
    listSaid.value = '';
    listOpen.value = true;
  });
}
const closeList = () => {
  listOpen.value = false;
};

// the game has no trash can among its UI sprites: one drawn in its icon style, in the ink colour, the slits in the
// button face's colour (style.css .ptrash .slit)
const TrashIcon = () => (
  <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
    <path fill="currentColor" d="M9.2 2.5h5.6l.9 1.9H20a1 1 0 0 1 1 1V7H3V5.4a1 1 0 0 1 1-1h4.3z" />
    <path fill="currentColor" d="M4.8 8.5h14.4l-1.1 12a2 2 0 0 1-2 1.8H7.9a2 2 0 0 1-2-1.8z" />
    <path class="slit" d="M9.6 11.5v7.4M14.4 11.5v7.4" stroke-width="1.8" stroke-linecap="round" />
  </svg>
);
interface RowProps {
  p: DeepReadonly<Preset>;
  current: readonly number[];
  onView: (name: string) => void;
}
function PresetRow({ p, current, onView }: RowProps) {
  const cur = sameSkills(p.groups, current);
  return (
    // biome-ignore lint/a11y/useSemanticElements: a fieldset would bring its border and min-width; the row is the game's list row, named for its buttons
    <div class={cur ? 'prow cur' : 'prow'} role="group" aria-label={p.name}>
      {/* the name and count, the whole left of the row: a click shows the preset's skills */}
      <button class="pview" type="button" aria-haspopup="dialog" onClick={() => onView(p.name)}>
        <span class="pname">{p.name}</span>
        <span class="pmeta">{skillCount(p.groups.length)}</span>
        {cur ? <span class="pcur">current</span> : null}
      </button>
      <div class="pbtns">
        <button
          class="btn"
          type="button"
          onClick={() => {
            loadPreset(p.groups);
            closeList();
          }}
        >
          Load
        </button>
        {/* as Save: a preset is never emptied */}
        <button
          class="btn"
          type="button"
          disabled={!current.length}
          title={current.length ? '' : 'Add target skills first'}
          onClick={() => {
            listSaid.value = overwritePreset(p.name);
          }}
        >
          Overwrite
        </button>
        <button
          class="btn ptrash"
          type="button"
          aria-label="Delete"
          title="Delete"
          onClick={() => {
            listSaid.value = deletePreset(p.name);
          }}
        >
          <TrashIcon />
        </button>
      </div>
    </div>
  );
}
function PresetList({ onSave, onView }: { onSave: () => void; onView: (name: string) => void }) {
  const presets = fields.presets.value,
    current = fields.targets.value.map((t) => t.group);
  return (
    <Dialog id="preset-dlg" cls="prdlg" labelledBy="pr-title" open={listOpen.value} onClose={closeList}>
      <div class="dlg">
        <div class="row between dhead">
          <h3 id="pr-title">Target skill presets</h3>
          <button class="btn dclose" type="button" onClick={closeList}>
            Done
          </button>
        </div>
        <div class="prlist" id="pr-list">
          {presets.length ? null : <p class="hint">No presets yet.</p>}
          {presets.map((p) => (
            <PresetRow key={p.name} p={p} current={current} onView={onView} />
          ))}
        </div>
        <p class="hint" id="pr-msg" role="status">
          {listSaid}
        </p>
        <div class="row prfoot">
          <button
            class="btn primary"
            id="pr-save"
            type="button"
            aria-haspopup="dialog"
            disabled={!current.length}
            title={current.length ? '' : 'Add target skills first'}
            onClick={onSave}
          >
            Save current skills
          </button>
        </div>
      </div>
    </Dialog>
  );
}
// Save the current skills: the name offered ("Preset N+1"), selected when it opens; Enter or Save saves
interface Saving {
  open: boolean;
  /** the name in the box */
  name: string;
  /** what its status line says */
  said: string;
}
function SaveDialog({ saving, onChange }: { saving: Saving; onChange: (next: Saving) => void }) {
  const input = useRef<HTMLInputElement>(null);
  // after the dialog's own layout effect has shown it (a child's runs first)
  useLayoutEffect(() => {
    if (saving.open) must(input.current, 'the name box is mounted').select();
  }, [saving.open]);
  const close = () => onChange({ ...saving, open: false });
  const save = () => {
    const done = saveCurrent(saving.name);
    if (!done.saved) {
      onChange({ ...saving, said: done.why });
      return;
    }
    listSaid.value = done.said;
    close();
  };
  return (
    <Dialog id="pr-new" cls="prdlg prsmall" labelledBy="prn-title" open={saving.open} onClose={close}>
      <div class="dlg">
        <div class="row between dhead">
          <h3 id="prn-title">Save the current skills</h3>
        </div>
        <label class="prlab" for="prn-name">
          Preset name
        </label>
        <input
          type="text"
          id="prn-name"
          maxlength={40}
          autocomplete="off"
          ref={input}
          value={saving.name}
          onInput={(e) => onChange({ ...saving, name: e.currentTarget.value })}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              e.preventDefault();
              save();
            }
          }}
        />
        <p class="hint" id="prn-msg" role="status">
          {saving.said}
        </p>
        <div class="row prfoot">
          <button class="btn primary" id="prn-save" type="button" onClick={save}>
            Save
          </button>
          <button class="btn dclose" type="button" onClick={close}>
            Cancel
          </button>
        </div>
      </div>
    </Dialog>
  );
}
// a preset's skills as the editor's tiles; Rename opens its name for editing (Enter or Save name saves it, Escape
// leaves it), Close
interface ViewProps {
  /** the preset on view (null: closed) */
  name: string | null;
  /** counts the openings: each starts the view afresh */
  n: number;
  onRenamed: (name: string) => void;
  onClose: () => void;
}
function ViewDialog({ name, n, onRenamed, onClose }: ViewProps) {
  return (
    <Dialog id="pr-view" cls="prdlg prview" labelledBy="prv-title" open={name !== null} onClose={onClose}>
      <ViewBody key={n} name={name} onRenamed={onRenamed} onClose={onClose} />
    </Dialog>
  );
}
function ViewBody({ name, onRenamed, onClose }: Omit<ViewProps, 'n'>) {
  const p = fields.presets.value.find((x) => x.name === name);
  const [renaming, setRenaming] = useState(false),
    [typed, setTyped] = useState(''),
    [said, setSaid] = useState(''),
    input = useRef<HTMLInputElement>(null);
  useLayoutEffect(() => {
    if (!renaming) return;
    const box = must(input.current, 'the name box is mounted');
    box.focus();
    box.select();
  }, [renaming]);
  const startRename = () => {
    setTyped(p?.name ?? '');
    setRenaming(true);
  };
  const commit = () => {
    const done = renamePreset(must(p, 'Save name is in the open preset').name, typed);
    if (done && !done.renamed) {
      setSaid(done.why);
      return;
    }
    if (done) {
      setSaid(done.said);
      onRenamed(done.name);
      listSaid.value = '';
    }
    setRenaming(false);
  };
  return (
    <div class="dlg">
      <div class="row between dhead">
        <h3 id="prv-title">{p?.name}</h3>
      </div>
      <div class="targets prvtiles" id="prv-tiles">
        {p?.groups.map((group) => (
          <TargetTile key={group} group={group} />
        ))}
        {p && !p.groups.length ? <p class="hint">No skills.</p> : null}
      </div>
      <div class="prvren" id="prv-ren" hidden={!renaming}>
        <label class="prlab" for="prv-name">
          New name
        </label>
        <input
          type="text"
          id="prv-name"
          maxlength={40}
          autocomplete="off"
          ref={input}
          value={typed}
          onInput={(e) => setTyped(e.currentTarget.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              e.preventDefault();
              commit();
            } else if (e.key === 'Escape') {
              e.preventDefault();
              e.stopPropagation(); // the rename, not the popup
              setRenaming(false);
            }
          }}
        />
      </div>
      <p class="hint" id="prv-msg" role="status">
        {said}
      </p>
      <div class="row prfoot">
        <button class="btn" id="prv-rename" type="button" onClick={renaming ? commit : startRename}>
          {renaming ? 'Save name' : 'Rename'}
        </button>
        <button class="btn dclose" type="button" onClick={onClose}>
          Close
        </button>
      </div>
    </div>
  );
}
// The three dialogs. Each opening of Save or of a view starts it afresh.
export function PresetDialogs() {
  const [saving, setSaving] = useState<Saving>({ open: false, name: '', said: '' }),
    [viewing, setViewing] = useState<{ n: number; name: string | null }>({ n: 0, name: null });
  const save = () => {
    if (!fields.targets.peek().length) return;
    setSaving({ open: true, name: suggestName(), said: '' });
  };
  return (
    <>
      <PresetList onSave={save} onView={(name) => setViewing({ n: viewing.n + 1, name })} />
      <SaveDialog saving={saving} onChange={setSaving} />
      <ViewDialog
        n={viewing.n}
        name={viewing.name}
        onRenamed={(name) => setViewing({ ...viewing, name })}
        onClose={() => setViewing({ ...viewing, name: null })}
      />
    </>
  );
}
