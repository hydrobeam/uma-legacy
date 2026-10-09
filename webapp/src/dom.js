// DOM shorthands shared by every module.
export const $ = (id) => document.getElementById(id);
export function esc(s) { return String(s).replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]); }
// a spinner whose turn continues across redraws (status lines are rebuilt every second; a new element would restart its
// 0.8 s turn from zero): its animation starts in phase with the clock
export const spinHTML = () => `<span class="spin" aria-hidden="true" style="animation-delay:-${Math.round(performance.now() % 800)}ms"></span>`;
// an element with a class and its text (text null or undefined: none)
export const el = (tag, cls, text) => { const e = document.createElement(tag); if (cls) e.className = cls; if (text != null) e.textContent = text; return e; };
// odds as "1 in N" (N rounded from 100 up, one decimal below)
export const fmtIn = (p) => p > 0 ? '1 in ' + (1 / p >= 100 ? Math.round(1 / p).toLocaleString() : (1 / p).toFixed(1)) : 'never';
// a click on the dialog's backdrop (the dialog element itself is the target) closes it
export const backdropCloses = (d) => d.addEventListener('click', (e) => { if (e.target === d) d.close(); });
