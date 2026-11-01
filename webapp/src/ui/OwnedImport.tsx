// ---------- Import umadump: the file button of the Umas and the Support cards panels ----------
// Either takes card_data.json (owned outfits) or support_card_data.json (owned support cards), or both (owned.js);
// `onSaid` gets what the import says, for the panel's status (`useSaid`).
import { useState } from 'preact/hooks';
import { importFiles } from '../owned.ts';

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
    <label
      class="btn filebtn"
      title="Owned outfits (card_data.json) or support cards (support_card_data.json) from umadump: replaces the marks here"
    >
      Import umadump
      <input
        type="file"
        id={id}
        accept=".json,application/json"
        multiple
        onChange={(e) => {
          void importFiles(e.currentTarget).then(onSaid);
        }}
      />
    </label>
  );
}
