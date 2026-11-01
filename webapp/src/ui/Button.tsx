// ---------- the game's buttons: a 9-slice sprite for a face ----------
// `plain`: the white rectangle (an action), `primary`: the green one (the main action, white outlined text), `slot`: the
// race schedule's slot frame (a race banner as a button). The sprites are vite/images.mjs's (--btn-w, ...), drawn by the
// `nine-*` utility from the theme's --nine-* tokens (styles/theme.css), whose faces dark mode swaps. `size` is the type
// size (the theme's scale). A caller's `class` places the button (margins, alignment, width), never restyles it
// (Biome's noTailwindRestyledComponents).
import type { ButtonHTMLAttributes, ComponentChildren, InputHTMLAttributes } from 'preact';
import { tv } from 'tailwind-variants/lite';
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
