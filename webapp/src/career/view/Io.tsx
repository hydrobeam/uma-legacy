// The veterans' Export and Paste popups (ui/Dialog): Export shows the text of the veterans, the plan and the job (Copy,
// Download), made when it opens (career/io.ts `exportText`); Paste takes a text to import (io.ts `importText`), its box
// empty at each opening. The career page's layout draws each in its box.
import { signal } from '@preact/signals';
import { useLayoutEffect, useRef } from 'preact/hooks';
import { must } from '../../must.ts';
import { Dialog } from '../../ui/Dialog.tsx';
import { copyExport, downloadExport, exportText, importText } from '../io.ts';

// ---------- Export
const exported = signal({ open: false, text: '', msg: '' });
/** Export (the footer's button): the text of the stores now */
export function openExport() {
  exported.value = { open: true, text: exportText(), msg: '' };
}
const closeExport = () => {
  exported.value = { ...exported.peek(), open: false };
};
const sayExport = (msg: string) => {
  exported.value = { ...exported.peek(), msg };
};
export function ExportDialog() {
  const { open, text, msg } = exported.value,
    box = useRef<HTMLTextAreaElement>(null);
  return (
    <Dialog id="vx-dlg" labelledBy="vx-title" open={open} onClose={closeExport}>
      <div class="dlg">
        <div class="row between dhead">
          <h3 id="vx-title">Export veterans</h3>
          <button class="btn dclose" type="button" onClick={closeExport}>
            Close
          </button>
        </div>
        <p>
          Your veterans, the plan and the solver job the planner sends for them. Copy the text to keep or share it;
          Paste brings it back.
        </p>
        <textarea
          ref={box}
          id="vx-text"
          rows={12}
          spellcheck={false}
          readonly
          aria-label="Exported veterans, plan and solver job"
          value={text}
        />
        <div class="row">
          <button
            class="btn"
            type="button"
            id="vx-copy"
            onClick={() => copyExport(must(box.current, 'the text box is drawn'), sayExport)}
          >
            Copy
          </button>
          <button
            class="btn"
            type="button"
            id="vx-dl"
            onClick={() => {
              downloadExport(text);
              sayExport('Downloaded.');
            }}
          >
            Download
          </button>
          <span class="hint" id="vx-msg" role="status">
            {msg}
          </span>
        </div>
      </div>
    </Dialog>
  );
}

// ---------- Paste
const pasteOpen = signal(false);
/** Paste (the veterans dialog's button) */
export function openPaste() {
  pasteOpen.value = true;
}
const closePaste = () => {
  pasteOpen.value = false;
};
export function PasteDialog() {
  const open = pasteOpen.value,
    box = useRef<HTMLTextAreaElement>(null);
  // each opening starts empty
  // biome-ignore lint/correctness/useExhaustiveDependencies: once per opening (`open` is a module signal's value, which Biome reads as constant)
  useLayoutEffect(() => {
    if (open && box.current) box.current.value = '';
  }, [open]);
  return (
    <Dialog id="paste-dlg" labelledBy="paste-title" open={open} onClose={closePaste}>
      <div class="dlg">
        <div class="row between dhead">
          <h3 id="paste-title">Paste veterans</h3>
          <button class="btn dclose" type="button" onClick={closePaste}>
            Close
          </button>
        </div>
        <p>
          The contents of umadump's <code>trained_chara_data.json</code> (yours) or <code>rental_chara_data.json</code>{' '}
          (the veterans you can borrow), the text from this page's Export, or an uma.moe lineage planner export (one
          veteran with her parents, added after yours). Your veterans merge with those already here (the file's version
          replaces the same veteran, nothing is removed); a borrow list replaces the earlier one, since it is who lends
          today; an Export replaces every import. Veterans you made here stay.
        </p>
        <textarea ref={box} id="paste-text" rows={10} spellcheck={false} aria-label="umadump, Export or uma.moe text" />
        <div class="row">
          <button
            class="btn"
            type="button"
            id="paste-load"
            onClick={() => {
              if (importText(must(box.current, 'the text box is drawn').value)) closePaste();
            }}
          >
            Import
          </button>
        </div>
      </div>
    </Dialog>
  );
}
