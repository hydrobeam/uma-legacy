// DOM shorthands shared by every module.
export const $ = (id) => document.getElementById(id);
export function esc(s) { return String(s).replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]); }
// a spinner whose turn continues across redraws (status lines are rebuilt every second; a new element would restart its
// 0.8 s turn from zero): its animation starts in phase with the clock
export const spinHTML = () => `<span class="spin" aria-hidden="true" style="animation-delay:-${Math.round(performance.now() % 800)}ms"></span>`;
