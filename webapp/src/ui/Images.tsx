// ---------- portraits, card art, skill icons, the runner (their files: images.ts) ----------
// Portraits, card art and skill icons are lazy unless marked eager: the browser fetches one only when it scrolls near
// the view. One that keeps state (an image that fails) is keyed by its subject at the call site, so a reused one never
// shows the last subject's state. Each carries a data-* mark (data-av, data-card, data-icon, data-grade, data-spinner,
// data-scen-logo) by which the checks find it.
import { useState } from 'preact/hooks';
import { gameData } from '../data.ts';
import type { Card } from '../data.ts';
import { GRADE_LETTER } from '../grades.ts';
import type { Grade as GradeCode } from '../grades.ts';
import { KIND_ABBR, RARITY_CLASS, RUN_CYCLE_MS, initials, runnerSrc, skillIconSrc, srcFor } from '../images.ts';
import { cx, tv } from 'tailwind-variants/lite';
import type { VariantProps } from 'tailwind-variants/lite';

// An uma's portrait: a circle (`round`, --s across, set inline) or the roster's square tile (`tile`, its box's width),
// her initials on her hue (--h, set inline) in the mono face while there is no picture, a faint edge just outside it;
// `dim`, greyed out (an outfit marked not owned)
const avLook = tv({
  base: 'bg-av font-data text-ink outline-img-edge inline-grid shrink-0 place-items-center overflow-hidden align-middle leading-none font-semibold outline',
  variants: {
    shape: {
      round: 'size-(--s) rounded-full text-(length:--fs)',
      tile: 'text-display aspect-square h-auto w-full rounded-xl',
    },
    dim: { true: 'opacity-45 grayscale' },
  },
  defaultVariants: { shape: 'round' },
});
type UmaAvProps = VariantProps<typeof avLook> & {
  uma: number | null;
  outfit: number | null | undefined;
  size: number;
  /** where it sits (margins, alignment): layout utilities only */
  cls?: string | undefined;
  /** a name read out for her (she becomes an image) */
  label?: string;
  title?: string;
  /** in the first screenful (the roster's first rows): fetched at once, at high priority, not lazily */
  eager?: boolean;
};
/** an uma's portrait (her outfit's, else her own, else a mob's), else her initials on her hue */
export function UmaAv({ uma, outfit, size, shape, dim, cls, label, title, eager = false }: UmaAvProps) {
  const id = uma ?? 0;
  const tries = [outfit && srcFor('outfit', outfit), srcFor('uma', id), srcFor('mob', id)].filter(
    (src): src is string => !!src,
  );
  const [failed, setFailed] = useState(0); // how many of `tries` failed to load
  const src = tries[failed];
  return (
    // biome-ignore lint/a11y/useAriaPropsSupportedByRole: the label comes with role="img" (a portrait read out as her name)
    <span
      class={avLook({ shape, dim, class: cls })}
      data-av=""
      role={label ? 'img' : undefined}
      aria-label={label}
      title={title}
      style={{ '--s': `${size}px`, '--fs': `${Math.round(size * 0.38)}px`, '--h': String((id * 47) % 360) }}
    >
      {src ? (
        <img
          class="size-full object-cover"
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
// A card's tile: its art (its own frame drawn in), else its type's letters in white on its type's colour (--k, set
// inline) in a ring of its rarity's (--rr); --s across (`fixed`) or its box's width (`fill`, the card list's); `dim`,
// greyed out (a card not owned, or banned). data-card says which it shows, "art" or "letters".
const cardLook = tv({
  base: 'font-data inline-grid shrink-0 place-items-center overflow-hidden align-middle leading-none font-bold text-white',
  variants: {
    art: { true: 'relative', false: 'bg-kind ring-rarity ring-2' },
    shape: {
      fixed: 'size-(--s) rounded-sm text-(length:--fs)',
      fill: 'text-title aspect-square h-auto w-full min-w-0 rounded-md',
    },
    dim: { true: 'opacity-45 grayscale' },
  },
  defaultVariants: { shape: 'fixed' },
});
type CardAvProps = Omit<VariantProps<typeof cardLook>, 'art'> & { c: Card | null; size: number; titled?: boolean };
/** a card's art with its type's icon, else its type in letters; `titled`: its own hover names */
export function CardAv({ c, size, titled = true, shape, dim }: CardAvProps) {
  const kind: string = c?.kind || 'other',
    src = c && srcFor('card', c.id),
    [failed, setFailed] = useState(false);
  const art = !!src && !failed,
    typeIcon = art && srcFor('ui', `type_${kind}`),
    abbr = KIND_ABBR[kind as keyof typeof KIND_ABBR] || '·';
  return (
    <span
      class={cardLook({ art, shape, dim })}
      data-card={art ? 'art' : 'letters'}
      style={{
        '--s': `${size}px`,
        '--fs': `${Math.max(7, Math.round(size * 0.3))}px`,
        '--k': `var(--color-k-${kind})`,
        '--rr': c ? `var(--color-r-${RARITY_CLASS[c.rar as keyof typeof RARITY_CLASS] || 'r'})` : 'transparent',
      }}
      title={titled ? (c ? c.name : '') : undefined}
    >
      {art ? (
        <>
          <img class="size-full object-cover" alt="" loading="lazy" src={src} onError={() => setFailed(true)} />
          {typeIcon && (
            <i
              class="pointer-events-none absolute top-0 right-[3%] aspect-square w-[26.4%] bg-contain bg-center bg-no-repeat"
              data-ctype=""
              title={titled ? kind : undefined}
              style={{ backgroundImage: `url(${typeIcon})` }}
            />
          )}
        </>
      ) : (
        abbr
      )}
    </span>
  );
}
/** a scenario's logo (the game's title art, img/scen/{id}, drawn whole inside its box), else, without it or when it
 *  fails, a badge with its name on a light shade of its colour (dark text on the light tint, in either theme); `tile`:
 *  the scenario menu's (the art at its box's left, a larger badge) */
export function ScenLogo({
  sc,
  title,
  tile = false,
  cls,
}: {
  sc: { id: number; name: string; tint: string };
  title?: string;
  tile?: boolean;
  /** its box's size, where its place sets it */
  cls?: string;
}) {
  const src = srcFor('scen', sc.id),
    [failed, setFailed] = useState(false);
  return src && !failed ? (
    <span class={cx('relative block shrink-0', cls)} data-scen-logo="" aria-hidden="true" title={title}>
      <img
        class={cx('absolute inset-0 block size-full object-contain', tile && 'object-left')}
        alt=""
        src={src}
        onError={() => setFailed(true)}
      />
    </span>
  ) : (
    <span class={cx('grid shrink-0 place-items-center', cls)} data-scen-logo="" aria-hidden="true" title={title}>
      <b
        class={cx(
          'rounded-md text-center leading-tight font-bold',
          tile ? 'text-body px-3 py-1.5' : 'text-note px-1.75 py-0.75',
        )}
        style={{ background: sc.tint, color: '#18202e' }}
      >
        {sc.name}
      </b>
    </span>
  );
}
// a grade's box: an aptitude row's (smaller on a phone) or the win table's column head
const GRADE_BOX = { row: 'h-5.75 w-5 max-sm:h-4.25 max-sm:w-3.5', head: 'h-5 w-4.25' } as const;
/** a grade letter (code 1..8 = G..S): the game's sprite, or the letter in the game's brown; data-grade is its letter */
export function Grade({ g, size = 'row' }: { g: GradeCode; size?: keyof typeof GRADE_BOX }) {
  const src = srcFor('ui', `grade_${g - 1}`),
    letter = GRADE_LETTER[g];
  return src ? (
    <img
      class={cx('inline-block shrink-0 object-contain', GRADE_BOX[size])}
      src={src}
      alt={letter}
      data-grade={letter}
    />
  ) : (
    <b
      class={cx('text-title text-gm-ink inline-grid shrink-0 place-items-center font-black', GRADE_BOX[size])}
      data-grade={letter}
    >
      {letter}
    </b>
  );
}
/** a skill's icon (its gold's: `gold`), or nothing when the page has none. `cls` sets its size and margins (layout
 *  only) in place of the default: 20 px, in a line of text before its name. */
export function SkillIcon({ group, gold = false, cls }: { group: number; gold?: boolean; cls?: string | undefined }) {
  const src = skillIconSrc(group, gold);
  return src ? (
    <img
      class={cx('inline shrink-0', cls ?? 'mr-1.5 size-5 align-[-4px]')}
      data-icon="skill"
      alt=""
      loading="lazy"
      src={src}
    />
  ) : null;
}
/** the runner while a search works (her strip of 18 frames, 30 px; the strip set inline with its size, held on its
 *  first frame for reduced motion), else a spinner; its stride starts in phase with the clock when it appears, and it
 *  stays the same element while the status line around it changes. `cls` sets the runner's margins (layout only) in
 *  place of the default, which overhangs its line a little. */
export function Runner({ cls }: { cls?: string | undefined } = {}) {
  const [at] = useState(() => performance.now()),
    src = runnerSrc();
  return src ? (
    <span
      class={cx(
        'animate-runner inline-block size-(--s) shrink-0 align-middle motion-reduce:animate-none',
        cls ?? '-mt-1.75 -mb-1.25',
      )}
      data-spinner="runner"
      aria-hidden="true"
      style={`--s:30px;background:url(${src}) 0 0 / auto 100% no-repeat;animation-delay:-${Math.round(at % RUN_CYCLE_MS)}ms`}
    />
  ) : (
    <span
      class="border-line border-t-accent animate-spinner inline-block size-3.5 rounded-full border-2 motion-reduce:animate-none"
      data-spinner="spin"
      aria-hidden="true"
      style={`animation-delay:-${Math.round(at % 800)}ms`}
    />
  );
}
