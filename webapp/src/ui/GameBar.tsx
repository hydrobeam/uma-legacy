// ---------- the game's green header bar: white outlined text on green, stripes at its left fading out ----------
// Its right end is cut on a slant, with two slanted notches. `section` (the default): a section's or a box's title bar
// (the career page's sections, "Sparks", the loop page's Swaps); `card`: the title on top of a card, centred, its top
// corners round (Try a run's veteran).
import type { ComponentChildren } from 'preact';
import { tv } from 'tailwind-variants/lite';
import type { VariantProps } from 'tailwind-variants/lite';

const look = tv({
  base: 'art-bar font-game leading-title tracking-title text-shadow-outline clip-bar after:art-notch relative flex items-center font-black text-white after:absolute after:inset-y-0 after:right-3.25 after:w-3.75 after:-skew-x-14 after:opacity-80',
  variants: {
    shape: {
      section: 'text-title rounded-l-sm py-0.75 pr-10 pl-3',
      card: 'text-heading rounded-t-xl px-3 pt-1.75 pb-2 text-center',
    },
  },
  defaultVariants: { shape: 'section' },
});
type GameBarProps = VariantProps<typeof look> & {
  /** the element: a heading where it heads a section, else a div or p */
  as?: 'h2' | 'h3' | 'div' | 'p';
  id?: string;
  /** where it sits: layout utilities only */
  class?: string;
  children: ComponentChildren;
};
/** a green header bar */
export function GameBar({ as: Tag = 'h3', shape, id, class: place, children }: GameBarProps) {
  return (
    <Tag id={id} class={look({ shape, class: place })}>
      {children}
    </Tag>
  );
}
