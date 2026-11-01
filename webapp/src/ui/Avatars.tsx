// images.js's element builders as components, each the twin of its element version until that one's last caller
// converts (frontend.md §3).
import { useState } from 'preact/hooks';
import { gameData } from '../data.js';
import { initials, srcFor } from '../images.js';

// umaAv: her portrait (her outfit's, else her character's, else a mob's) in a disc of her hue, the next source on an
// error; her initials where there is no art or every source fails. chara null: an unknown uma, drawn as id 0
type UmaAvProps = { chara: number | null; outfit?: number | null; size: number };
export function UmaAv({ chara, outfit, size }: UmaAvProps) {
  const id = chara ?? 0,
    tries = [outfit && srcFor('outfit', outfit), srcFor('uma', id), srcFor('mob', id)].filter(Boolean),
    [at, setAt] = useState(0);
  const style = { '--s': `${size}px`, '--fs': `${Math.round(size * 0.38)}px`, '--h': String((id * 47) % 360) };
  return (
    <span class="av" style={style}>
      {at < tries.length ? (
        <img key={at} alt="" loading="lazy" src={tries[at] || undefined} onError={() => setAt(at + 1)} />
      ) : (
        initials(gameData().names.get(id) || '?')
      )}
    </span>
  );
}
