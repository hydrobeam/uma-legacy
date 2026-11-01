// ---------- the component twins of images.js's element builders: portraits, card art, skill icons, the runner ----------
// Each draws the markup its element builder draws (images.js says what each is), for the views that are components.
// The builders stay for the views that are not yet: images.js is shared with the career page, whose bundle has no
// Preact, so a builder can't render its twin. Each twin names its builder and each builder its twin: an edit to one
// goes to the other, until step 8c drops the builders. A twin that keeps state (an image that fails) is keyed by its
// subject at the call site, so a reused one never shows the last subject's state.
import { useState } from 'preact/hooks';
import { SKILLICON, SKILLICON_GOLD, gameData } from '../data.js';
import type { Card } from '../data.js';
import { KIND_ABBR, RARITY_CLASS, RUN_CYCLE_MS, initials, runnerSrc, srcFor } from '../images.js';

/** an uma's portrait (her outfit's, else her own, else a mob's), else her initials on her hue (images.js umaAv) */
export function UmaAv({ uma, outfit, size }: { uma: number | null; outfit: number | null | undefined; size: number }) {
  const id = uma ?? 0;
  const tries = [outfit && srcFor('outfit', outfit), srcFor('uma', id), srcFor('mob', id)].filter(
    (src): src is string => !!src,
  );
  const [failed, setFailed] = useState(0); // how many of `tries` failed to load
  const src = tries[failed];
  return (
    <span
      class="av"
      style={{ '--s': `${size}px`, '--fs': `${Math.round(size * 0.38)}px`, '--h': String((id * 47) % 360) }}
    >
      {src ? (
        <img alt="" loading="lazy" src={src} onError={() => setFailed(failed + 1)} />
      ) : (
        initials(gameData().names.get(id) || '?')
      )}
    </span>
  );
}
/** a card's art with its type's icon, else its type in letters (images.js cardAv); `titled`: its own hover names */
export function CardAv({ c, size, titled = true }: { c: Card | null; size: number; titled?: boolean }) {
  const kind: string = c?.kind || 'other',
    src = c && srcFor('card', c.id),
    [failed, setFailed] = useState(false);
  const art = !!src && !failed,
    typeIcon = art && srcFor('ui', `type_${kind}`),
    abbr = KIND_ABBR[kind as keyof typeof KIND_ABBR] || '·';
  return (
    <span
      class={art ? 'cav art' : 'cav'}
      style={{
        '--s': `${size}px`,
        '--fs': `${Math.max(7, Math.round(size * 0.3))}px`,
        '--k': `var(--k-${kind})`,
        '--rr': c ? `var(--r-${RARITY_CLASS[c.rar as keyof typeof RARITY_CLASS] || 'r'})` : 'transparent',
      }}
      title={titled ? (c ? c.name : '') : undefined}
    >
      {art ? (
        <>
          <img alt="" loading="lazy" src={src} onError={() => setFailed(true)} />
          {typeIcon && (
            <i class="ctype" title={titled ? kind : undefined} style={{ backgroundImage: `url(${typeIcon})` }} />
          )}
        </>
      ) : (
        abbr
      )}
    </span>
  );
}
/** a skill's icon (its gold's: `gold`), or nothing when the page has none (images.js skillIcon) */
export function SkillIcon({ group, gold = false }: { group: number; gold?: boolean }) {
  const src = srcFor('skill', (gold ? SKILLICON_GOLD : SKILLICON)[group]);
  return src ? <img class="sic" alt="" loading="lazy" src={src} /> : null;
}
/** the runner while a search works, else a spinner (images.js runnerHTML); its stride starts in phase with the clock
 *  when it appears, and it stays the same element while the status line around it changes */
export function Runner() {
  const [at] = useState(() => performance.now()),
    src = runnerSrc();
  return src ? (
    <span
      class="runner"
      aria-hidden="true"
      style={`background-image:url(${src});animation-delay:-${Math.round(at % RUN_CYCLE_MS)}ms`}
    />
  ) : (
    <span class="spin" aria-hidden="true" style={`animation-delay:-${Math.round(at % 800)}ms`} />
  );
}
