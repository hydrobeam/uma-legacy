// ---------- Export / import: the settings and the result on show as text to copy, or another's text to load ----------
// The text is made when the dialog opens (io.js exportText) and stays while it is open, editable: Load reads what is in
// the box then (io.js readImport). Settings for the other data set are stored under its keys and the page reloads into
// it; the template's controls that aren't components yet are painted again after a load.
import { signal } from '@preact/signals';
import { useRef, useState } from 'preact/hooks';
import { $input } from '../dom.ts';
import { ERA } from '../saved.ts';
import { importSettings, importToEra, settings } from '../settings.ts';
import { Dialog } from '../ui/Dialog.tsx';
import { must } from '../must.ts';
import { exportText, readImport } from './io.ts';
import { loopView, resultsStale } from './state.ts';
import { RenderGuard } from '../ui/RenderGuard.tsx';

// the export made at the last opening (`n` counts them: each opening starts the box afresh), and whether it is open
const opened = signal({ n: 0, text: '', open: false });
export function openExport() {
  opened.value = {
    n: opened.peek().n + 1,
    text: exportText(settings.peek(), loopView.peek().lastResult, resultsStale.peek()),
    open: true,
  };
}
const close = () => {
  opened.value = { ...opened.peek(), open: false };
};
export function ExportDialog() {
  const { n, text, open } = opened.value;
  return (
    <Dialog id="ex-dlg" labelledBy="ex-title" open={open} onClose={close}>
      <RenderGuard input={opened} what="Export">
        <ExportBody key={n} made={text} />
      </RenderGuard>
    </Dialog>
  );
}
function ExportBody({ made }: { made: string }) {
  const [text, setText] = useState(made),
    [message, setMessage] = useState('');
  const box = useRef<HTMLTextAreaElement>(null);
  const copy = () => {
    const fallback = () => {
      const area = must(box.current, 'the text box is mounted');
      area.focus();
      area.select();
      setMessage('Selected. Press Ctrl+C (or ⌘C) to copy.');
    };
    try {
      navigator.clipboard.writeText(text).then(() => setMessage('Copied.'), fallback);
    } catch {
      fallback();
    }
  };
  const load = () => {
    let read: ReturnType<typeof readImport>;
    try {
      read = readImport(text, settings.peek().presets);
    } catch {
      setMessage('That text is not an export from this page.');
      return;
    }
    if (read.era !== ERA) {
      importToEra(read.era, read.loaded);
      location.reload();
      return;
    }
    importSettings(read.loaded);
    // what the template draws itself (the rest reads the settings)
    $input('c-peruma').checked = settings.peek().perUma !== false;
    $input('x-auto').checked = settings.peek().auto !== false;
    setMessage(
      `Settings loaded.${read.bad.length ? ` Not understood, so left as they were: ${read.bad.join(', ')}.` : ''}`,
    );
  };
  return (
    <div class="dlg">
      <div class="row between dhead">
        <h3 id="ex-title">Export or import</h3>
        <button type="button" class="btn" id="ex-close" onClick={close}>
          Close
        </button>
      </div>
      <p>
        The text below holds your settings and the results on screen. Copy it to share or to paste into a chat. To load
        someone else's settings, paste their text here and press Load settings.
      </p>
      <textarea
        ref={box}
        id="ex-text"
        rows={12}
        spellcheck={false}
        aria-label="Exported settings and results"
        value={text}
        onInput={(e) => setText(e.currentTarget.value)}
      />
      <div class="row">
        <button type="button" class="btn" id="ex-copy" onClick={copy}>
          Copy
        </button>
        <button type="button" class="btn" id="ex-load" onClick={load}>
          Load settings
        </button>
        <span class="hint" id="ex-msg">
          {message}
        </span>
      </div>
    </div>
  );
}
