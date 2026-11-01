// ---------- the game's skill tile with its icon and name (the career page's targets, Skill Details) ----------
import { tv } from 'tailwind-variants/lite';

/** a skill tile: the game's white sprite, its gold one (`gold`), or greyed (`off`: not acquired); `row`, a target's
 *  row's button (opening Skill Details), `popup`, the tile at the top of Skill Details */
export const skillTile = tv({
  base: 'flex min-w-0 items-center text-left font-bold',
  variants: {
    tone: {
      white: 'nine-skill-white text-skill-ink',
      gold: 'nine-skill-gold text-gm-gold-ink',
      off: 'bg-gm-off text-gm-off-ink inset-ring-gm-edge rounded-md inset-ring-2',
    },
    size: {
      row: 'focus-visible:outline-accent min-h-9.5 w-full cursor-pointer gap-1.75 pt-0.75 pr-3 pb-1.75 pl-1.75 hover:brightness-104 focus-visible:outline-3 focus-visible:outline-offset-2',
      popup: 'min-h-11.5 gap-2 pt-1.25 pr-3 pb-2.25 pl-2',
    },
  },
  defaultVariants: { tone: 'white', size: 'popup' },
});
