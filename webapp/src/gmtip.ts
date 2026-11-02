// ---------- the planner pages' tooltips (html.game): an element's title as the game's balloon ----------
// game-style tooltips: after a short hover (or on keyboard focus) an element's title shows as the game's balloon; the
// title is lifted only while the balloon shows (no native tooltip twice), and put back when it goes, unless it was drawn
// again meanwhile (a component's render, the status line's marks): that one is current. Lifting the title is the one
// write it makes into elements it does not own (a component's too: Preact diffs a title against its own last render,
// never the element, so a title lifted and put back unchanged is invisible to it). The balloon is its own element in
// <body>, shown in the top layer (a manual popover) so it stays over an open modal dialog without being put inside it.
const tipEl = document.createElement('div');
// the game's balloon: bold brown on white in a green rim, its arrow over the element (under it with data-up); fixed at
// the place set below, so a popover's own centring (inset, margin) is undone
tipEl.className =
  'font-game text-note leading-title text-gm-ink bg-gm-panel border-gm-bright2 shadow-tip before:border-b-gm-bright2 data-up:before:border-t-gm-bright2 pointer-events-none fixed inset-auto z-1000 m-0 max-w-75 overflow-visible rounded-xl border-2 px-2.5 py-1.5 font-bold whitespace-pre-line before:absolute before:-top-2 before:left-[calc(50%-7px)] before:border-x-7 before:border-t-0 before:border-b-8 before:border-x-transparent data-up:before:top-auto data-up:before:-bottom-2 data-up:before:border-t-8 data-up:before:border-b-0';
tipEl.id = 'gm-tip';
tipEl.setAttribute('role', 'tooltip');
tipEl.hidden = true;
const popover = 'showPopover' in tipEl; // (a browser without popovers shows it under an open modal dialog)
if (popover) tipEl.popover = 'manual';
const GAP = 10,
  EDGE = 8; // px: between the element and the balloon; the least between the balloon and the window's sides
let tipFor: HTMLElement | null = null;
// the elements whose title is held while the balloon shows, and their titles
const lifted = new Map<HTMLElement, string>();
let tipTimer: number | undefined;
function tipShow(target: HTMLElement) {
  if (!target.isConnected || !target.title) return;
  // her title and every title round her (a card tile's own, round its picture's): with any left the browser would show it
  tipFor = target;
  tipEl.textContent = target.title;
  for (let e: HTMLElement | null = target; e && e !== document.documentElement; e = e.parentElement) {
    if (e.hasAttribute('title')) {
      lifted.set(e, e.title);
      e.removeAttribute('title');
    }
  }
  if (!tipEl.isConnected) document.body.appendChild(tipEl);
  tipEl.hidden = false;
  if (popover) {
    if (tipEl.matches(':popover-open')) tipEl.hidePopover();
    tipEl.showPopover(); // (again: on top of a dialog opened since)
  }
  const rect = target.getBoundingClientRect(),
    width = tipEl.offsetWidth,
    height = tipEl.offsetHeight;
  const below = rect.bottom + GAP + height < innerHeight;
  tipEl.style.left = `${Math.max(EDGE, Math.min(rect.left + rect.width / 2 - width / 2, innerWidth - width - EDGE))}px`;
  tipEl.style.top = `${below ? rect.bottom + GAP : rect.top - height - GAP}px`;
  tipEl.toggleAttribute('data-up', !below);
}
function tipHide() {
  clearTimeout(tipTimer);
  for (const [e, title] of lifted) if (!e.hasAttribute('title')) e.title = title; // (drawn again meanwhile: kept)
  lifted.clear();
  tipFor = null;
  tipEl.hidden = true;
  if (popover && tipEl.matches(':popover-open')) tipEl.hidePopover();
}
// moving onto a child of the element showing its balloon keeps it (its title is lifted, so a lookup for [title] would
// find an ancestor, put the title back and the browser would show its own tooltip beside ours)
// (a mouse or focus event lands on an element; lib.dom types its target as a bare EventTarget, hence the casts)
document.addEventListener('mouseover', (e) => {
  if (tipFor?.contains(e.target as Element)) return;
  const target = (e.target as Element).closest?.('[title]');
  tipHide();
  if (target) tipTimer = setTimeout(() => tipShow(target), 400);
});
document.addEventListener('mouseout', (e) => {
  const keep = tipFor || (e.target as Element).closest?.('[title]');
  if (!e.relatedTarget || !keep?.contains(e.relatedTarget as Element)) tipHide();
});
document.addEventListener('focusin', (e) => {
  const target = (e.target as Element).closest?.('[title]');
  tipHide();
  if (target && (e.target as Element).matches(':focus-visible')) tipShow(target);
});
document.addEventListener('focusout', tipHide);
addEventListener('scroll', tipHide, true);
document.addEventListener('pointerdown', tipHide); // scrolling or a tap hides it
