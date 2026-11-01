// ---------- a deck card's button (the loop page's results, the career page's decks) ----------
import { tv } from 'tailwind-variants/lite';

/** a deck card's button: its art alone (its name, hidden, for the screen reader), else its name in a box (a pal's in
 *  green, a borrowed card's in blue); a borrowed card's art in the friend frame; `swap`, a swap row's smaller card */
export const chip = tv({
  base: 'group/chip text-caption focus-visible:outline-accent flex cursor-pointer flex-col items-stretch gap-1 rounded-md text-left not-has-data-[card=art]:border not-has-data-[card=art]:px-1.5 not-has-data-[card=art]:py-1 focus-visible:outline-2 focus-visible:outline-offset-3',
  variants: {
    tag: {
      plain: 'not-has-data-[card=art]:border-line',
      pal: 'not-has-data-[card=art]:border-req text-req',
      // (the frame drawn over the art and 3 px round it; cqw: the card's width, the art's height)
      borrowed:
        'not-has-data-[card=art]:border-accent text-accent after:nine-friend @container relative after:pointer-events-none after:absolute after:-top-0.75 after:-left-0.75 after:hidden after:h-[calc(100cqw+15px)] after:w-[calc(100%+6px)] has-data-[card=art]:after:block',
    },
    swap: { true: 'w-16 max-md:w-13' },
  },
});
