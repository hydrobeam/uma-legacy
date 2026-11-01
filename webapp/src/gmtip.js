// ---------- the planner pages' tooltips (html.game): an element's title as the game's balloon ----------
// game-style tooltips: after a short hover (or on keyboard focus) an element's title shows as the game's balloon; the
// title is lifted only while the balloon shows (no native tooltip twice), and put back when it goes, unless it was drawn
// again meanwhile (a component's render, the status line's marks): that one is current. Lifting the title is the one
// write it makes into elements it does not own (a component's too: Preact diffs a title against its own last render,
// never the element, so a title lifted and put back unchanged is invisible to it). The balloon is its own element in
// <body>, shown in the top layer (a manual popover) so it stays over an open modal dialog without being put inside it.
const tipEl = document.createElement('div');
tipEl.className = 'gmtip';
tipEl.setAttribute('role', 'tooltip');
tipEl.hidden = true;
const popover = 'showPopover' in tipEl; // (a browser without popovers shows it under an open modal dialog)
if (popover) tipEl.popover = 'manual';
const GAP = 10,
  EDGE = 8; // px: between the element and the balloon; the least between the balloon and the window's sides
/** @type {HTMLElement | null} */
let tipFor = null;
/** the elements whose title is held while the balloon shows, and their titles @type {Map<HTMLElement, string>} */
const lifted = new Map();
/** @type {number | undefined} */
let tipTimer;
function tipShow(target) {
  if (!target.isConnected || !target.title) return;
  // her title and every title round her (a card tile's own, round its picture's): with any left the browser would show it
  tipFor = target;
  tipEl.textContent = target.title;
  for (let e = target; e && e !== document.documentElement; e = e.parentElement) {
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
  tipEl.classList.toggle('up', !below);
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
document.addEventListener('mouseover', (/** @type {TargetedEvent} */ e) => {
  if (tipFor?.contains(e.target)) return;
  const target = e.target.closest?.('[title]');
  tipHide();
  if (target) tipTimer = setTimeout(() => tipShow(target), 400);
});
document.addEventListener(
  'mouseout',
  (/** @type {TargetedEvent<Element, MouseEvent & {relatedTarget: Element | null}>} */ e) => {
    const keep = tipFor || e.target.closest?.('[title]');
    if (!e.relatedTarget || !keep?.contains(e.relatedTarget)) tipHide();
  },
);
document.addEventListener('focusin', (/** @type {TargetedEvent} */ e) => {
  const target = e.target.closest?.('[title]');
  tipHide();
  if (target && e.target.matches(':focus-visible')) tipShow(target);
});
document.addEventListener('focusout', tipHide);
addEventListener('scroll', tipHide, true);
document.addEventListener('pointerdown', tipHide); // scrolling or a tap hides it
