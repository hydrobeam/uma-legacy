// ---------- portraits, card art, skill icons, the runner (their files: model/images.ts) ----------
// Portraits, card art and skill icons are lazy unless marked eager: the browser fetches one only when it scrolls near
// the view. One that keeps state (an image that fails) is keyed by its subject at the call site, so a reused one never
// shows the last subject's state. Each carries a data-* mark (data-av, data-card, data-icon, data-grade, data-spinner,
// data-scen-logo) by which the checks find it.
import { useState } from 'preact/hooks';
import { gameData } from '../model/data.ts';
import type { Card } from '../model/data.ts';
import { Grade } from '../gen/solver.ts';
import { KIND_ABBR, RARITY_CLASS, RUN_CYCLE_MS, initials, runnerSrc, skillIconSrc, srcFor } from '../model/images.ts';
import { cx, tv } from 'tailwind-variants/lite';
import type { VariantProps } from 'tailwind-variants/lite';

// An uma's portrait: a circle (`round`, --s across, set inline), the roster's square tile (`tile`, its box's width), or
// a result's face button (ui's `face/` group: loop/Results.tsx), brighter under the pointer: `card`, its corners
// rounded (a deck's title), `swap`, rounder and 52 px on a phone (a swap row's); `trainee`, a circle 72 px on a phone
// (the career tree's trainee); `vet`, a circle a veteran tile's portrait across (its --p: veterans/VetCell.tsx);
// `insp`, a circle 48 px on a narrow phone (the Inspiration view's: veterans/InspView.tsx); her initials on her hue
// (--h, set inline) in the mono face while there is no picture, a faint edge just outside it; `ring`, a loop member's
// white ring (`member`), a tree node's white and cream one (`node`) or the Inspiration view's, without its shadow
// (`insp`); `dim`, greyed out: `always`, `faint` (fainter: a legacy without the skill, in Skill Details), or `unowned`
// while her roster tile is marked not owned (the theme's `unowned:`)
const avLook = tv({
  base: 'bg-av font-data text-ink outline-img-edge inline-grid shrink-0 place-items-center overflow-hidden align-middle leading-none font-semibold outline',
  variants: {
    shape: {
      round: 'size-(--s) rounded-full text-(length:--fs)',
      tile: 'text-display aspect-square h-auto w-full rounded-xl',
      card: 'size-(--s) rounded-md text-(length:--fs) group-hover/face:brightness-106',
      swap: 'size-(--s) rounded-xl text-(length:--fs) group-hover/face:brightness-106 max-md:size-13',
      trainee: 'size-(--s) rounded-full text-(length:--fs) max-md:size-18',
      vet: 'size-(--p) rounded-full text-(length:--fs)',
      insp: 'size-(--s) rounded-full text-(length:--fs) max-sm:size-12',
    },
    ring: { member: 'shadow-portrait', node: 'shadow-node-face', insp: 'shadow-insp-face' },
    dim: {
      always: 'opacity-45 grayscale',
      unowned: 'unowned:opacity-45 unowned:grayscale',
      faint: 'opacity-30 grayscale',
    },
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
export function UmaAv({ uma, outfit, size, shape, ring, dim, cls, label, title, eager = false }: UmaAvProps) {
  const id = uma ?? 0;
  const tries = [outfit && srcFor('outfit', outfit), srcFor('uma', id), srcFor('mob', id)].filter(
    (src): src is string => !!src,
  );
  const [failed, setFailed] = useState(0); // how many of `tries` failed to load
  const src = tries[failed];
  return (
    // biome-ignore lint/a11y/useAriaPropsSupportedByRole: the label comes with role="img" (a portrait read out as her name)
    <span
      class={avLook({ shape, ring, dim, class: cls })}
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
// inline) in a ring of its rarity's (--rr); --s across (`fixed`) or its box's width (`fill`, the card list's), or a
// result's deck card (`deck`: its art the width of its button, brighter under the pointer (the button's `chip/`
// group), its letters --s across; `friend`, a borrowed card's art square on the friend frame's pink, room under it for
// the frame's band); `dim`, greyed out: `always` (a card the scenario bans, one not owned in a result), or `unpressed`
// while the toggle round it (a `group`) is not pressed (a card not owned). data-card says which it shows, "art" or
// "letters".
const cardLook = tv({
  base: 'font-data inline-grid shrink-0 place-items-center overflow-hidden align-middle leading-none font-bold text-white',
  variants: {
    art: { true: 'relative', false: 'bg-kind ring-rarity ring-2' },
    shape: {
      fixed: 'size-(--s) rounded-sm text-(length:--fs)',
      fill: 'text-title aspect-square h-auto w-full min-w-0 rounded-md',
      deck: '',
    },
    friend: { true: '' },
    dim: { always: 'opacity-45 grayscale', unpressed: 'group-not-on:opacity-45 group-not-on:grayscale' },
  },
  compoundVariants: [
    { shape: 'deck', art: false, class: 'size-(--s) rounded-sm text-(length:--fs)' },
    {
      shape: 'deck',
      art: true,
      friend: false,
      class: 'aspect-square h-auto w-full rounded-md group-hover/chip:brightness-107',
    },
    {
      shape: 'deck',
      art: true,
      friend: true,
      class: 'bg-friend mb-3 aspect-square h-auto w-full rounded-none group-hover/chip:brightness-107',
    },
  ],
  defaultVariants: { shape: 'fixed', friend: false },
});
type CardAvProps = Omit<VariantProps<typeof cardLook>, 'art'> & { c: Card | null; size: number; titled?: boolean };
/** a card's art with its type's icon, else its type in letters; `titled`: its own hover names */
export function CardAv({ c, size, titled = true, shape, friend, dim }: CardAvProps) {
  const kind: string = c?.kind || 'other',
    src = c && srcFor('card', c.id),
    [failed, setFailed] = useState(false);
  const art = !!src && !failed,
    typeIcon = art && srcFor('ui', `type_${kind}`),
    abbr = KIND_ABBR[kind as keyof typeof KIND_ABBR] || '·';
  return (
    <span
      class={cardLook({ art, shape, friend, dim })}
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
export function GradeBadge({ g, size = 'row' }: { g: Grade; size?: keyof typeof GRADE_BOX }) {
  const src = srcFor('ui', `grade_${g - 1}`),
    letter = Grade[g];
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
export function SkillIcon({
  group,
  gold = false,
  dim = false,
  cls,
}: {
  group: number;
  gold?: boolean;
  /** greyed (a skill not acquired) */
  dim?: boolean;
  cls?: string | undefined;
}) {
  const src = skillIconSrc(group, gold);
  return src ? (
    <img
      class={cx('inline shrink-0', dim && 'opacity-60 grayscale', cls ?? 'mr-1.5 size-5 align-[-4px]')}
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
 *  place of the default, which overhangs its line a little; `game`, the spinner in the game's colours (the career
 *  page's status line), else the site's. */
const SPIN = {
  site: 'border-line border-t-accent animate-spinner inline-block size-3.5 rounded-full border-2 motion-reduce:animate-none',
  game: 'border-gm-line border-t-gm-green2 animate-spinner inline-block size-3.5 rounded-full border-2 motion-reduce:animate-none',
} as const;
export function Runner({ cls, game = false }: { cls?: string | undefined; game?: boolean } = {}) {
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
      class={SPIN[game ? 'game' : 'site']}
      data-spinner="spin"
      aria-hidden="true"
      style={`animation-delay:-${Math.round(at % 800)}ms`}
    />
  );
}
