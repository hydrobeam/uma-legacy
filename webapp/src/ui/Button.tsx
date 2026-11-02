// ---------- the game's buttons: a 9-slice sprite for a face ----------
// `plain`: the white rectangle (an action), `primary`: the green one (the main action, white outlined text), `slot`: the
// race schedule's slot frame (a race banner as a button). The sprites are vite/images.mjs's (--btn-w, ...), drawn by the
// `nine-*` utility from the theme's --nine-* tokens (styles/theme.css), whose faces dark mode swaps. `size` is the type
// size (the theme's scale). A caller's `class` places the button (margins, alignment, width), never restyles it
// (Biome's noTailwindRestyledComponents).
// `EnhButton`: the game's Enhance menu button, art over a frame (the Umas, Support cards and Veteran Umamusume buttons).
import type { ButtonHTMLAttributes, ComponentChildren, InputHTMLAttributes } from 'preact';
import { cx, tv } from 'tailwind-variants/lite';
import type { VariantProps } from 'tailwind-variants/lite';

const look = tv({
  base: 'focus-visible:outline-accent cursor-pointer font-bold focus-visible:outline-3 focus-visible:outline-offset-2 active:translate-y-px enabled:hover:brightness-104 disabled:opacity-55 disabled:grayscale',
  variants: {
    variant: {
      plain: 'nine-btn-w text-gm-ink min-h-10.5 px-5 pt-2 pb-2.75 max-md:min-h-11',
      primary: 'nine-btn-g text-shadow-outline min-h-10.5 pt-2 pr-6.5 pb-2.75 pl-5 text-white max-md:min-h-11',
      slot: 'nine-slot-frame text-gm-ink grid justify-items-center gap-0.5 px-1.5 pt-1.5 pb-1.75 text-left',
    },
    size: { caption: 'text-caption', note: 'text-note', ui: 'text-ui', body: 'text-body', title: 'text-title' },
  },
  defaultVariants: { variant: 'plain', size: 'ui' },
});
/** a button's type size, one of the theme's */
export type ButtonSize = NonNullable<VariantProps<typeof look>['size']>;
type Look = VariantProps<typeof look> & {
  /** where it sits: layout utilities only */
  class?: string | undefined;
};
/** a button's classes (also a file input's label's) */
const buttonClass = ({ variant, size, class: place }: Look = {}) => look({ variant, size, class: place });

type ButtonProps = Look & Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'class' | 'className' | 'size'>;
/** a game button (type="button" unless given) */
export function Button({ variant, size, class: place, type = 'button', ...rest }: ButtonProps) {
  return <button type={type} class={buttonClass({ variant, size, class: place })} {...rest} />;
}

type FileInput = Omit<InputHTMLAttributes<HTMLInputElement>, 'type' | 'role' | 'class' | 'className' | 'title'>;
/** a file input as a plain button: the label is its face, the input covers it unseen (the label shows its focus) */
export function FileButton({ title, children, ...input }: { title: string; children: ComponentChildren } & FileInput) {
  return (
    <label
      title={title}
      class={buttonClass({
        class: 'focus-within:outline-accent relative focus-within:outline-2 focus-within:outline-offset-2',
      })}
    >
      {children}
      <input type="file" class="absolute inset-0 w-full cursor-pointer opacity-0" {...input} />
    </label>
  );
}

/** a count on an Enhance button, as its tooltip (none until the entry has counted) */
export interface Badge {
  title: string;
}
// ---------- the game's Enhance menu buttons: the game's art (its frame and white lower band, the art spilling over its
// top edge; the drop shadow follows that shape), the label on the band with a ▶ beside its first line, the count owned
// in its tooltip (the band holds one line: a count under the label pushed the label onto the art). Umas and Support
// cards are square, one size on both pages (the label's margins a little wider on the career page); Veteran Umamusume
// is wide, its two-line label right of the art, as wide as the row of Enhance buttons
const ENH_ART = { umas: 'art-enh-umas', cards: 'art-enh-cards', vets: 'art-enh-vets' } as const;
const ENH = {
  career: {
    button: 'h-28 w-35',
    label:
      'after:border-l-enh-ink text-body text-enh-ink absolute right-6 bottom-2.25 left-2 text-center leading-tight font-black whitespace-nowrap after:absolute after:top-1 after:-right-3.25 after:border-y-6 after:border-l-8 after:border-y-transparent',
  },
  loop: {
    button: 'h-28 w-35',
    label:
      'after:border-l-enh-ink text-body text-enh-ink absolute right-5 bottom-2.25 left-1.5 text-center leading-tight font-black whitespace-nowrap after:absolute after:top-1 after:-right-3.25 after:border-y-6 after:border-l-8 after:border-y-transparent',
  },
  wide: {
    button: 'col-span-full h-18 w-71.5',
    label:
      'after:border-l-enh-ink text-body text-enh-ink absolute top-1/2 right-8.5 left-[44%] -translate-y-1/2 text-center leading-tight font-black *:block after:absolute after:top-1/2 after:-right-5.5 after:-mt-1.5 after:border-y-6 after:border-l-8 after:border-y-transparent',
  },
} as const;
/** an Enhance button (it opens a dialog): `art` its picture, `size` the page's square one or the wide one */
export function EnhButton({
  id,
  art,
  size,
  label,
  name,
  badge,
  onClick,
}: {
  id: string;
  art: keyof typeof ENH_ART;
  size: keyof typeof ENH;
  label: ComponentChildren;
  /** its name for assistive tech, when the label's words are not */
  name?: string | undefined;
  badge?: Badge | undefined;
  onClick: (() => void) | undefined;
}) {
  return (
    <button
      class={cx(
        'group focus-visible:outline-gm-bright2 relative inline-grid rounded-xl focus-visible:outline-3 focus-visible:outline-offset-2 active:translate-y-0.5',
        ENH[size].button,
      )}
      type="button"
      id={id}
      aria-haspopup="dialog"
      aria-label={name}
      title={badge?.title}
      onClick={onClick}
    >
      <span
        class={cx(
          'drop-shadow-enh group-hover:drop-shadow-enh-glow absolute inset-0 group-hover:brightness-106',
          ENH_ART[art],
        )}
        aria-hidden="true"
      />
      <span class={ENH[size].label}>{label}</span>
    </button>
  );
}
