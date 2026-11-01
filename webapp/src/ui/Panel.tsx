// ---------- the game's white panel: a box on the page or in a popup (white, its faint edge, round corners, a soft shadow) ----------
// Its look only, as one class list a box's own layout joins (`cx(panel, 'grid gap-2 p-2.5')`): the boxes that wear it
// lay out their contents differently, and a component's class may only place it. Dark mode's panel and edge are the
// theme's.
export const panel = 'bg-gm-panel border-gm-edge shadow-panel rounded-2xl border';
