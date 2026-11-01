// ---------- the game's buttons: a 9-slice sprite for a face ----------
// `plain`: the white rectangle (an action), `primary`: the green one (the main action, white outlined text), `slot`: the
// race schedule's slot frame (a race banner as a button). The sprites are vite/images.mjs's (--btn-w, ...), drawn by the
// `nine-*` utility from the theme's --nine-* tokens (styles/theme.css), which dark mode redefines. `size` is the type
// size. A caller's `class` places the button (margins, alignment, width), never restyles it (Biome's
// noTailwindRestyledComponents).
import type { ButtonHTMLAttributes, ComponentChildren, InputHTMLAttributes } from 'preact';
import { variants } from './variants.ts';

export type ButtonVariant = 'plain' | 'primary' | 'slot';
export type ButtonSize = 'xs' | 'sm' | 'md' | 'base' | 'lg';

const look = variants({
  base: 'focus-visible:outline-accent cursor-pointer rounded-none bg-transparent font-bold focus-visible:outline-3 focus-visible:outline-offset-2 active:translate-y-px enabled:hover:brightness-104 disabled:opacity-55 disabled:grayscale',
  variants: {
    variant: {
      plain: 'nine-btn-w text-gm-ink min-h-10.5 px-5 pt-2 pb-2.75 max-md:min-h-11',
      primary: 'nine-btn-g text-shadow-outline min-h-10.5 pt-2 pr-6.5 pb-2.75 pl-5 text-white max-md:min-h-11',
      slot: 'nine-slot-frame text-gm-ink grid justify-items-center gap-0.5 px-1.5 pt-1.5 pb-1.75 text-left',
    },
    size: { xs: 'text-xs', sm: 'text-sm', md: 'text-md', base: 'text-base', lg: 'text-lg' },
  },
  defaults: { variant: 'plain', size: 'md' },
});
interface Look {
  variant?: ButtonVariant | undefined;
  size?: ButtonSize | undefined;
  /** where it sits: layout utilities only */
  class?: string | undefined;
}
/** a button's classes (also a file input's label's) */
const buttonClass = ({ variant, size, class: place }: Look = {}) => look({ variant, size }, place);
/** the green button's text is white on the game's green, exempt from the contrast audit like the game's other art */
const art = (variant: ButtonVariant | undefined) => (variant === 'primary' ? '' : undefined);

type ButtonProps = Look & Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'class' | 'className' | 'size'>;
/** a game button (type="button" unless given) */
export function Button({ variant, size, class: place, type = 'button', ...rest }: ButtonProps) {
  return <button type={type} data-art={art(variant)} class={buttonClass({ variant, size, class: place })} {...rest} />;
}

type FileInput = Omit<InputHTMLAttributes<HTMLInputElement>, 'type' | 'role' | 'class' | 'className' | 'title'>;
/** a file input as a plain button: the label is its face, the input covers it unseen (the label shows its focus) */
export function FileButton({ title, children, ...input }: { title: string; children: ComponentChildren } & FileInput) {
  return (
    <label
      title={title}
      class={buttonClass({
        class: 'relative focus-within:outline-2 focus-within:outline-accent focus-within:outline-offset-2',
      })}
    >
      {children}
      <input type="file" class="absolute inset-0 w-full cursor-pointer opacity-0" {...input} />
    </label>
  );
}
