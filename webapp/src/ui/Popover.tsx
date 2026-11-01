// ---------- a popup's frame: the box a panel, menu or note opens in, over the page ----------
// Its look only (a popup's place, size and layout are its own, joined through `class`): `game`, the game's popup (its
// rounded face and brown ink on white, a green rim: a deck card's or member's panel, the wins menu, the "+" list);
// `panel`, the game's white panel with a deeper shadow (the scenario menu); `plain`, the site's menu (a sort order, the
// rank's note). Dark mode's colours are the theme's.
import { tv } from 'tailwind-variants/lite';

export const popover = tv({
  variants: {
    frame: {
      game: 'bg-gm-panel border-gm-bright2 font-game text-gm-ink shadow-pop rounded-xl border-2',
      panel: 'bg-gm-panel border-gm-edge shadow-pop rounded-xl border',
      plain: 'bg-panel border-line text-ink shadow-menu rounded-md border',
    },
  },
});
