// ---------- the planner pages' tooltips (html.game): an element's title as the game's balloon ----------
// game-style tooltips: after a short hover (or on keyboard focus) an element's title shows as the game's balloon; the
// title is lifted only while the balloon shows (no native tooltip twice), and put back when it goes
const tipEl = document.createElement('div'); tipEl.className = 'gmtip'; tipEl.setAttribute('role', 'tooltip'); tipEl.hidden = true;
const GAP = 10, EDGE = 8;   // px: between the element and the balloon; the least between the balloon and the window's sides
let tipFor = null, tipTimer = null, lifted = [];   // lifted: the elements whose title is held while the balloon shows
function tipShow(target) {
  if (!target.isConnected || !target.title) return;
  // her title and every title round her (a card tile's own, round its picture's): with any left the browser would show it
  tipFor = target; tipEl.textContent = target.title;
  for (let e = target; e && e !== document.documentElement; e = e.parentElement) {
    if (e.hasAttribute('title')) { e.dataset.tip = e.title; e.removeAttribute('title'); lifted.push(e); }
  }
  (target.closest('dialog[open]') || document.body).appendChild(tipEl); tipEl.hidden = false;
  const rect = target.getBoundingClientRect(), width = tipEl.offsetWidth, height = tipEl.offsetHeight;
  const below = rect.bottom + GAP + height < innerHeight;
  tipEl.style.left = `${Math.max(EDGE, Math.min(rect.left + rect.width / 2 - width / 2, innerWidth - width - EDGE))}px`;
  tipEl.style.top = `${below ? rect.bottom + GAP : rect.top - height - GAP}px`; tipEl.classList.toggle('up', !below);
}
function tipHide() {
  clearTimeout(tipTimer);
  for (const e of lifted) { if (e.dataset.tip !== undefined) e.title = e.dataset.tip; delete e.dataset.tip; }
  lifted = []; tipFor = null; tipEl.hidden = true;
}
// moving onto a child of the element showing its balloon keeps it (its title is lifted, so a lookup for [title] would
// find an ancestor, put the title back and the browser would show its own tooltip beside ours)
document.addEventListener('mouseover', (e) => {
  if (tipFor?.contains(e.target)) return;
  const target = e.target.closest?.('[title]');
  tipHide();
  if (target) tipTimer = setTimeout(() => tipShow(target), 400);
});
document.addEventListener('mouseout', (e) => {
  const keep = tipFor || e.target.closest?.('[title]');
  if (!e.relatedTarget || !keep?.contains(e.relatedTarget)) tipHide();
});
document.addEventListener('focusin', (e) => {
  const target = e.target.closest?.('[title]');
  tipHide();
  if (target && e.target.matches(':focus-visible')) tipShow(target);
});
document.addEventListener('focusout', tipHide);
addEventListener('scroll', tipHide, true); document.addEventListener('pointerdown', tipHide);   // scrolling or a tap hides it
