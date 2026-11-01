// ---------- portraits, card art, skill icons, the runner (their files: images.ts) ----------
// Portraits, card art and skill icons are lazy unless marked eager: the browser fetches one only when it scrolls near
// the view. One that keeps state (an image that fails) is keyed by its subject at the call site, so a reused one never
// shows the last subject's state.
import { useState } from 'preact/hooks';
import { gameData } from '../data.ts';
import type { Card } from '../data.ts';
import { GRADE_LETTER } from '../grades.ts';
import type { Grade as GradeCode } from '../grades.ts';
import { KIND_ABBR, RARITY_CLASS, RUN_CYCLE_MS, initials, runnerSrc, skillIconSrc, srcFor } from '../images.ts';

interface UmaAvProps {
  uma: number | null;
  outfit: number | null | undefined;
  size: number;
  /** a class beside `av` */
  cls?: string | undefined;
  /** a name read out for her (she becomes an image) */
  label?: string;
  title?: string;
  /** in the first screenful (the roster's first rows): fetched at once, at high priority, not lazily */
  eager?: boolean;
}
/** an uma's portrait (her outfit's, else her own, else a mob's), else her initials on her hue */
export function UmaAv({ uma, outfit, size, cls, label, title, eager = false }: UmaAvProps) {
  const id = uma ?? 0;
  const tries = [outfit && srcFor('outfit', outfit), srcFor('uma', id), srcFor('mob', id)].filter(
    (src): src is string => !!src,
  );
  const [failed, setFailed] = useState(0); // how many of `tries` failed to load
  const src = tries[failed];
  return (
    // biome-ignore lint/a11y/useAriaPropsSupportedByRole: the label comes with role="img" (a portrait read out as her name)
    <span
      class={cls ? `av ${cls}` : 'av'}
      role={label ? 'img' : undefined}
      aria-label={label}
      title={title}
      style={{ '--s': `${size}px`, '--fs': `${Math.round(size * 0.38)}px`, '--h': String((id * 47) % 360) }}
    >
      {src ? (
        <img
          alt=""
          loading={eager ? 'eager' : 'lazy'}
          fetchpriority={eager ? 'high' : undefined}
          src={src}
          onError={() => setFailed(failed + 1)}
        />
      ) : (
        initials(gameData().names.get(id) || '?')
      )}
    </span>
  );
}
/** a card's art with its type's icon, else its type in letters; `titled`: its own hover names */
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
/** a scenario's logo (the game's title art, img/scen/{id}, drawn whole inside its box), else, without it or when it
 *  fails, a badge with its name on a light shade of its colour (dark text on the light tint, in either theme) */
export function ScenLogo({ sc, title }: { sc: { id: number; name: string; tint: string }; title?: string }) {
  const src = srcFor('scen', sc.id),
    [failed, setFailed] = useState(false);
  return src && !failed ? (
    <span class="slogo art" aria-hidden="true" title={title}>
      <img alt="" src={src} onError={() => setFailed(true)} />
    </span>
  ) : (
    <span class="slogo" aria-hidden="true" title={title}>
      <b style={{ background: sc.tint, color: '#18202e' }}>{sc.name}</b>
    </span>
  );
}
/** a grade letter (code 1..8 = G..S): the game's sprite, or the letter */
export function Grade({ g }: { g: GradeCode }) {
  const src = srcFor('ui', `grade_${g - 1}`),
    letter = GRADE_LETTER[g];
  return src ? <img class="udg" src={src} alt={letter} /> : <b class={`udg txt g${letter}`}>{letter}</b>;
}
/** a skill's icon (its gold's: `gold`), or nothing when the page has none */
export function SkillIcon({ group, gold = false }: { group: number; gold?: boolean }) {
  const src = skillIconSrc(group, gold);
  return src ? <img class="sic" alt="" loading="lazy" src={src} /> : null;
}
/** the runner while a search works, else a spinner; its stride starts in phase with the clock
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
