// ---------- Import umadump: the file button of the Umas and the Support cards panels ----------
// Either takes card_data.json (owned outfits) or support_card_data.json (owned support cards), or both (owned.js);
// `onSaid` gets what the import says, for the panel's status.
import { importFiles } from '../owned.js';

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
