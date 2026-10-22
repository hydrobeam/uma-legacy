// DOM shorthands shared by every module.
export const $ = (id) => document.getElementById(id);
// text made safe to put in HTML markup or a double-quoted attribute
const ENTITY = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' };
export function esc(text) { return String(text).replace(/[&<>"]/g, ch => ENTITY[ch]); }
// a spinner whose turn continues across redraws (status lines are rebuilt every second; a new element would restart its
// 0.8 s turn from zero): its animation starts in phase with the clock
export const spinHTML = () => `<span class="spin" aria-hidden="true" style="animation-delay:-${Math.round(performance.now() % 800)}ms"></span>`;
// an element with a class and its text (text null or undefined: none)
export const el = (tag, cls, text) => {
  const element = document.createElement(tag);
  if (cls) element.className = cls;
  if (text != null) element.textContent = text;
  return element;
};
// odds as "1 in N" (N rounded from 100 up, one decimal below)
export const fmtIn = (p) => {
  if (!(p > 0)) return 'never';
  const runs = 1 / p;
  return `1 in ${runs >= 100 ? Math.round(runs).toLocaleString() : runs.toFixed(1)}`;
};
// a click on the dialog's backdrop (the dialog element itself is the target) closes it
export const backdropCloses = (dialog) => dialog.addEventListener('click', (e) => { if (e.target === dialog) dialog.close(); });
