// ---------- the game's switch: a native checkbox with role="switch", its label beside it ----------
// The track is the game's toggle sprite (--tog-off, --tog-on: vite/images.mjs), 9-sliced so its round ends stay
// semicircles round the knob (the theme's nine-tog-* tokens; dark mode swaps the off face); without the sprite, the
// line colour and the green. The knob is the input's ::after, sliding right when on. `size`: `md` (a panel's Owned)
// or `sm` (the loop page's status row: small type, a 20 px track, and on a phone a 44 px tall touch area round the label).
// The label is the switch's name; a caller's `class` places it.
import type { ComponentChildren, InputHTMLAttributes } from 'preact';
import { tv } from 'tailwind-variants/lite';
import type { VariantProps } from 'tailwind-variants/lite';

const label = tv({
  base: 'inline-flex cursor-pointer items-center select-none',
  variants: {
    size: {
      md: 'gap-2 font-semibold',
      sm: 'text-caption text-gm-ink gap-1.25 font-bold max-md:relative max-md:before:absolute max-md:before:-inset-x-1 max-md:before:-inset-y-3',
    },
  },
  defaultVariants: { size: 'md' },
});
const track = tv({
  base: 'after:bg-gm-knob after:shadow-knob not-checked:bg-line checked:bg-gm-bright2 relative m-0 flex-none cursor-pointer appearance-none rounded-full transition-colors after:absolute after:rounded-full after:transition-transform',
  variants: {
    size: {
      md: 'not-checked:nine-tog-off checked:nine-tog-on h-6 w-11.5 after:top-0.75 after:left-0.75 after:size-4.5 checked:after:translate-x-5.5',
      sm: 'not-checked:nine-tog-off-sm checked:nine-tog-on-sm h-5 w-9 after:top-0.5 after:left-0.5 after:size-4 checked:after:translate-x-4',
    },
  },
  defaultVariants: { size: 'md' },
});
type SwitchProps = VariantProps<typeof track> &
  Omit<InputHTMLAttributes<HTMLInputElement>, 'type' | 'role' | 'class' | 'className' | 'size' | 'title'> & {
    /** the label's tooltip */
    title?: string;
    /** where it sits: layout utilities only */
    class?: string;
    /** its visible name */
    children: ComponentChildren;
  };
/** a switch and its label */
export function Switch({ size, title, class: place, children, ...input }: SwitchProps) {
  return (
    <label class={label({ size, class: place })} title={title}>
      <input
        type="checkbox"
        // biome-ignore lint/a11y/useAriaPropsForRole: a native checkbox's checked state is the switch's (ARIA in HTML: no aria-checked on a checkbox)
        role="switch"
        class={track({ size })}
        {...input}
      />
      {children}
    </label>
  );
}
