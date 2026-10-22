// DOM shorthands the page modules share.
/** @type {(id: string) => HTMLElement} */
export const $ = (id) => document.getElementById(id);
// the same lookup for an element whose kind the caller knows (a dialog, a text or check box, a text area): no check is
// made, the type only tells the type checker which properties it has
/** @type {(id: string) => HTMLDialogElement} */
export const $dialog = (id) => /** @type {HTMLDialogElement} */ (document.getElementById(id));
/** @type {(id: string) => HTMLInputElement} */
export const $input = (id) => /** @type {HTMLInputElement} */ (document.getElementById(id));
/** @type {(id: string) => HTMLTextAreaElement} */
export const $textarea = (id) => /** @type {HTMLTextAreaElement} */ (document.getElementById(id));
// text made safe to put in HTML markup or a double-quoted attribute
const ENTITY = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' };
/** @param {unknown} text @returns {string} */
export function esc(text) { return String(text).replace(/[&<>"]/g, ch => ENTITY[ch]); }
// a spinner whose turn continues across redraws (status lines are rebuilt every second; a new element would restart its
// 0.8 s turn from zero): its animation starts in phase with the clock
export const spinHTML = () => `<span class="spin" aria-hidden="true" style="animation-delay:-${Math.round(performance.now() % 800)}ms"></span>`;
// an element with a class and its text (text null or undefined: none)
/**
 * @template {keyof HTMLElementTagNameMap} K
 * @param {K} tag
 * @param {string} [cls]
 * @param {string|number|null} [text]
 * @returns {HTMLElementTagNameMap[K]}
 */
export const el = (tag, cls, text) => {
  const element = document.createElement(tag);
  if (cls) element.className = cls;
  if (text != null) element.textContent = String(text);
  return element;
};
// odds as "1 in N" (N rounded from 100 up, one decimal below)
/** @type {(p: number) => string} */
export const fmtIn = (p) => {
  if (!(p > 0)) return 'never';
  const runs = 1 / p;
  return `1 in ${runs >= 100 ? Math.round(runs).toLocaleString() : runs.toFixed(1)}`;
};
// a click on the dialog's backdrop (the dialog element itself is the target) closes it
/** @param {HTMLDialogElement} dialog */
export const backdropCloses = (dialog) => dialog.addEventListener('click', (e) => { if (e.target === dialog) dialog.close(); });
