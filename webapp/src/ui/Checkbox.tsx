// ---------- the game's check box: a native checkbox drawn as the game's sprite (unticked, ticked) ----------
// The sprites are vite/images.mjs's (--chk-off, --chk-on), painted by the theme's art-chk-* tokens, whose unticked face
// dark mode swaps. A caller's `class` places it (margins, alignment), never restyles it. Its label is the caller's.
import type { InputHTMLAttributes } from 'preact';
import { tv } from 'tailwind-variants/lite';

const look = tv({
  base: 'not-checked:art-chk-off checked:art-chk-on focus-visible:outline-accent size-6 flex-none cursor-pointer appearance-none align-middle focus-visible:outline-3 focus-visible:outline-offset-2',
});
type CheckboxProps = Omit<InputHTMLAttributes<HTMLInputElement>, 'type' | 'role' | 'class' | 'className'> & {
  /** where it sits: layout utilities only */
  class?: string;
};
/** a check box */
export const Checkbox = ({ class: place, ...input }: CheckboxProps) => (
  <input type="checkbox" class={look({ class: place })} {...input} />
);
