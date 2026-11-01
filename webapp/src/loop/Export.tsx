// ---------- Export / import: the settings and the result on show as text to copy, or another's text to load ----------
// The text is made when the dialog opens (io.ts exportText) and stays while it is open, editable: Load reads what is in
// the box then (io.ts readImport). Settings for the other data set are stored under its keys and the page reloads into
// it; the layout's switches follow the settings loaded.
import { signal } from '@preact/signals';
import { useRef, useState } from 'preact/hooks';
import { ERA } from '../saved.ts';
import { importSettings, importToEra, settings } from '../settings.ts';
import { Dialog, DialogHead, DialogPanel } from '../ui/Dialog.tsx';
import { must } from '../must.ts';
import { exportText, readImport } from './io.ts';
import { loopView, resultsStale } from './state.ts';
import { RenderGuard } from '../ui/RenderGuard.tsx';
import { Button } from '../ui/Button.tsx';

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
    <Dialog size="xl" id="ex-dlg" labelledBy="ex-title" open={open} onClose={close}>
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
    setMessage(
      `Settings loaded.${read.bad.length ? ` Not understood, so left as they were: ${read.bad.join(', ')}.` : ''}`,
    );
  };
  return (
    <DialogPanel>
      <DialogHead id="ex-title" title="Export or import" closeId="ex-close" close="Close" onClose={close} />
      <p class="max-w-[75ch]">
        The text below holds your settings and the results on screen. Copy it to share or to paste into a chat. To load
        someone else's settings, paste their text here and press Load settings.
      </p>
      <textarea
        ref={box}
        class="border-line bg-bg font-data text-note text-ink w-full rounded-md border p-2"
        id="ex-text"
        rows={12}
        spellcheck={false}
        aria-label="Exported settings and results"
        value={text}
        onInput={(e) => setText(e.currentTarget.value)}
      />
      <div class="flex flex-wrap items-center gap-2">
        <Button id="ex-copy" onClick={copy}>
          Copy
        </Button>
        <Button id="ex-load" onClick={load}>
          Load settings
        </Button>
        <span class="text-note text-gm-ink2" id="ex-msg">
          {message}
        </span>
      </div>
    </DialogPanel>
  );
}
