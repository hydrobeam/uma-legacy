// ---------- a deck card's button (the loop page's results, the career page's decks) ----------
import { tv } from 'tailwind-variants/lite';

/** a deck card's button: its art alone (its name, hidden, for the screen reader), else its name in a box (a pal's in
 *  green, a borrowed card's in blue); a borrowed card's art in the friend frame; `swap`, a swap row's smaller card;
 *  `friends`, a borrowed card without art ringed in the game's pink (Try a run's Career Info, with FRIENDS_TAG) */
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
    friends: { true: 'not-has-data-[card=art]:ring-gm-pink2 not-has-data-[card=art]:ring-3' },
  },
});
/** the "Friends" tag at the foot of a borrowed card without art (`friends`); with art the friend frame marks her */
export const FRIENDS_TAG =
  'bg-gm-pink2 text-caption absolute -bottom-2 left-1/2 -translate-x-1/2 rounded-full px-1 leading-tight font-black whitespace-nowrap text-white group-has-data-[card=art]/chip:hidden';
