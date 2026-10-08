// ---------- the planner pages' tooltips (html.game): an element's title as the game's balloon ----------
// game-style tooltips: after a short hover (or on keyboard focus) an element's title shows as the game's balloon; the
// title is lifted only while the balloon shows (no native tooltip twice), and put back when it goes
const tipEl = document.createElement('div'); tipEl.className = 'gmtip'; tipEl.setAttribute('role', 'tooltip'); tipEl.hidden = true;
let tipFor = null, tipTimer = null;
function tipShow(t) {
  if (!t.isConnected || !t.title) return;
  tipFor = t; t.dataset.tip = t.title; t.removeAttribute('title'); tipEl.textContent = t.dataset.tip;
  (t.closest('dialog[open]') || document.body).appendChild(tipEl); tipEl.hidden = false;
  const r = t.getBoundingClientRect(), w = tipEl.offsetWidth, h = tipEl.offsetHeight;
  const below = r.bottom + 10 + h < innerHeight;
  tipEl.style.left = Math.max(8, Math.min(r.left + r.width / 2 - w / 2, innerWidth - w - 8)) + 'px';
  tipEl.style.top = (below ? r.bottom + 10 : r.top - h - 10) + 'px'; tipEl.classList.toggle('up', !below);
}
function tipHide() { clearTimeout(tipTimer); if (tipFor) { if (tipFor.dataset.tip) tipFor.title = tipFor.dataset.tip; delete tipFor.dataset.tip; tipFor = null; } tipEl.hidden = true; }
// moving onto a child of the element showing its balloon keeps it (its title is lifted, so a lookup for [title] would
// find an ancestor, put the title back and the browser would show its own tooltip beside ours)
document.addEventListener('mouseover', (e) => { if (tipFor && tipFor.contains(e.target)) return; const t = e.target.closest?.('[title]');
  if (!t) { tipHide(); return; } tipHide(); tipTimer = setTimeout(() => tipShow(t), 400); });
document.addEventListener('mouseout', (e) => { const keep = tipFor || e.target.closest?.('[title]'); if (!e.relatedTarget || !keep || !keep.contains(e.relatedTarget)) tipHide(); });
document.addEventListener('focusin', (e) => { const t = e.target.closest?.('[title]'); tipHide(); if (t && e.target.matches(':focus-visible')) tipShow(t); });
document.addEventListener('focusout', tipHide);
addEventListener('scroll', tipHide, true); document.addEventListener('pointerdown', tipHide);   // scrolling or a tap hides it
