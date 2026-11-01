// ---------- Import umadump: the file button of the Umas and the Support cards panels ----------
// Either takes card_data.json (owned outfits) or support_card_data.json (owned support cards), or both (owned.ts);
// `onSaid` gets what the import says, for the panel's status (`useSaid`).
import { useState } from 'preact/hooks';
import { importFiles } from '../owned.ts';
import { FileButton } from './Button.tsx';

/** the two panels' shared rows: the top row (its title for the panel's name alone: the dialog's head shows it) with All /
 *  None owned, Import and the count at the right; the filter row (the box and the order); the hint line */
export const OWNED_PANEL = {
  top: 'flex flex-wrap items-center justify-between gap-2',
  tools: 'flex flex-1 flex-wrap items-center gap-2',
  count: 'text-note text-gm-ink2 ml-auto self-center pl-2.5 tabular-nums',
  filters: 'flex flex-wrap items-center gap-1.5',
  hint: 'text-note text-gm-ink2',
} as const;
/** what the last import said; `n` counts the imports */
export interface Said {
  n: number;
  text: string;
}
// The panel's record of what its imports said, and the function that records one. The panel draws `text` keyed by `n`,
// so each import puts a new text in its status line, as a repeat of the same message did before the port too.
export function useSaid() {
  const [said, setSaid] = useState<Said>({ n: 0, text: '' });
  return [said, (text: string) => setSaid((last) => ({ n: last.n + 1, text }))] as const;
}

export function OwnedImport({ id, onSaid }: { id: string; onSaid: (said: string) => void }) {
  return (
    <FileButton
      title="Owned outfits (card_data.json) or support cards (support_card_data.json) from umadump: replaces the marks here"
      id={id}
      accept=".json,application/json"
      multiple
      onChange={(e) => {
        void importFiles(e.currentTarget).then(onSaid);
      }}
    >
      Import umadump
    </FileButton>
  );
}
