// ---------- the Presets dialog (presets.ts holds the edits): the list, Save current skills, a preset's view ----------
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
} from '../presets.ts';
import { must } from '../must.ts';
import { Dialog, DialogHead, DialogPanel } from './Dialog.tsx';
import { TargetTile, groupKeys } from './TargetTile.tsx';
import { Button } from './Button.tsx';
import { field } from './Field.ts';
import { cx } from 'tailwind-variants/lite';

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
// the dialogs' shared parts: a status line (empty until something is said), a name box's label over its box, the row of
// buttons at the foot
const SAID = 'text-note text-gm-ink2';
const NAME_LABEL = 'text-ui text-gm-ink2 block font-bold';
const FOOT = 'flex flex-wrap items-center gap-2';

// the game has no trash can among its UI sprites: one drawn in its icon style, in the ink colour, the slits in the
// button face's colour
const TrashIcon = () => (
  <svg class="text-gm-ink size-5" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
    <path fill="currentColor" d="M9.2 2.5h5.6l.9 1.9H20a1 1 0 0 1 1 1V7H3V5.4a1 1 0 0 1 1-1h4.3z" />
    <path fill="currentColor" d="M4.8 8.5h14.4l-1.1 12a2 2 0 0 1-2 1.8H7.9a2 2 0 0 1-2-1.8z" />
    <path class="stroke-trash-slit" d="M9.6 11.5v7.4M14.4 11.5v7.4" stroke-width="1.8" stroke-linecap="round" />
  </svg>
);
interface RowProps {
  p: DeepReadonly<Preset>;
  current: readonly number[];
  onView: (name: string) => void;
}
// A preset's row (the game's list row, the current targets' row tinted): its name and count, the whole left of the row,
// a click shows the preset's skills; its buttons at the right (under the name when the row is narrow)
function PresetRow({ p, current, onView }: RowProps) {
  const cur = sameSkills(p.groups, current);
  return (
    // biome-ignore lint/a11y/useSemanticElements: a fieldset would bring its border and min-width; the row is the game's list row, named for its buttons
    <div
      class={cx(
        'border-gm-line flex flex-wrap items-stretch overflow-hidden rounded-md border',
        cur ? 'bg-gm-current' : 'bg-gm-cell',
      )}
      role="group"
      aria-label={p.name}
    >
      <button
        class="hover:bg-gm-ink2/10 flex min-h-14 min-w-0 flex-[1_1_200px] flex-wrap items-center gap-x-2.5 gap-y-0.5 px-3 py-2.5 text-left focus-visible:-outline-offset-3"
        type="button"
        aria-haspopup="dialog"
        onClick={() => onView(p.name)}
      >
        <span class="text-gm-ink font-bold wrap-anywhere">{p.name}</span>
        <span class="text-note text-gm-ink2">
          {skillCount(p.groups.length)}
          <span aria-hidden="true"> ›</span>
        </span>
        {cur ? (
          <span class="text-caption text-gm-bright-ink bg-gm-green2 rounded-full px-2 font-bold">current</span>
        ) : null}
      </button>
      <div class="flex flex-wrap items-center gap-1.5 py-2 pr-2.5 pl-1 max-md:pl-2.5">
        <Button
          onClick={() => {
            loadPreset(p.groups);
            closeList();
          }}
        >
          Load
        </Button>
        {/* as Save: a preset is never emptied */}
        <Button
          disabled={!current.length}
          title={current.length ? '' : 'Add target skills first'}
          onClick={() => {
            listSaid.value = overwritePreset(p.name);
          }}
        >
          Overwrite
        </Button>
        <Button
          class="inline-flex items-center"
          aria-label="Delete"
          title="Delete"
          onClick={() => {
            listSaid.value = deletePreset(p.name);
          }}
        >
          <TrashIcon />
        </Button>
      </div>
    </div>
  );
}
// the list (its rows drawn only while it is open)
function PresetList({ onSave, onView }: { onSave: () => void; onView: (name: string) => void }) {
  const open = listOpen.value,
    presets = open ? fields.presets.value : [],
    current = fields.targets.value.map((t) => t.group);
  return (
    <Dialog size="xl" id="preset-dlg" labelledBy="pr-title" open={open} onClose={closeList}>
      <DialogPanel>
        <DialogHead id="pr-title" title="Target skill presets" close="Done" onClose={closeList} />
        <div class="grid gap-2" id="pr-list">
          {open && !presets.length ? <p class={SAID}>No presets yet.</p> : null}
          {presets.map((p) => (
            <PresetRow key={p.name} p={p} current={current} onView={onView} />
          ))}
        </div>
        <p class={SAID} id="pr-msg" role="status">
          {listSaid}
        </p>
        <div class={FOOT}>
          <Button
            variant="primary"
            id="pr-save"
            aria-haspopup="dialog"
            disabled={!current.length}
            title={current.length ? '' : 'Add target skills first'}
            onClick={onSave}
          >
            Save current skills
          </Button>
        </div>
      </DialogPanel>
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
    <Dialog size="sm" id="pr-new" labelledBy="prn-title" open={saving.open} onClose={close}>
      <DialogPanel>
        <DialogHead id="prn-title" title="Save the current skills" />
        <div class="grid gap-1">
          <label class={NAME_LABEL} for="prn-name">
            Preset name
          </label>
          <input
            type="text"
            id="prn-name"
            class={field.tap}
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
        </div>
        <p class={SAID} id="prn-msg" role="status">
          {saving.said}
        </p>
        <div class={FOOT}>
          <Button variant="primary" id="prn-save" onClick={save}>
            Save
          </Button>
          <Button onClick={close}>Cancel</Button>
        </div>
      </DialogPanel>
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
    <Dialog size="lg" id="pr-view" labelledBy="prv-title" open={name !== null} onClose={onClose}>
      <ViewBody key={n} name={name} onRenamed={onRenamed} onClose={onClose} />
    </Dialog>
  );
}
function ViewBody({ name, onRenamed, onClose }: Omit<ViewProps, 'n'>) {
  const p = fields.presets.value.find((x) => x.name === name),
    tileKeys = groupKeys(p?.groups ?? []);
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
    <DialogPanel>
      <DialogHead id="prv-title" title={p?.name} />
      <div class="grid grid-cols-2 gap-x-2.5 gap-y-2" id="prv-tiles">
        {p?.groups.map((group, at) => (
          <TargetTile key={tileKeys[at]} group={group} size="view" />
        ))}
        {p && !p.groups.length ? <p class={cx(SAID, 'col-span-full')}>No skills.</p> : null}
      </div>
      <div class="grid gap-1" id="prv-ren" hidden={!renaming}>
        <label class={NAME_LABEL} for="prv-name">
          New name
        </label>
        <input
          type="text"
          id="prv-name"
          class={field.tap}
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
      <p class={SAID} id="prv-msg" role="status">
        {said}
      </p>
      <div class={FOOT}>
        <Button id="prv-rename" onClick={renaming ? commit : startRename}>
          {renaming ? 'Save name' : 'Rename'}
        </Button>
        <Button onClick={onClose}>Close</Button>
      </div>
    </DialogPanel>
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
