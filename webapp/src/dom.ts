// DOM shorthands the page modules share.
// (a literal id is on the page: test/static.mjs checks every one against the templates of the pages that run the module)
export const $ = (id: string): HTMLElement => document.getElementById(id) as HTMLElement;
// the same lookup for an element whose kind the caller knows (a dialog, a text or check box, a text area): no check is
// made, the type only tells the type checker which properties it has
export const $dialog = (id: string): HTMLDialogElement => document.getElementById(id) as HTMLDialogElement;
export const $input = (id: string): HTMLInputElement => document.getElementById(id) as HTMLInputElement;
export const $textarea = (id: string): HTMLTextAreaElement => document.getElementById(id) as HTMLTextAreaElement;
// text made safe to put in HTML markup or a double-quoted attribute
const ENTITY = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' };
export function esc(text: unknown): string {
  return String(text).replace(/[&<>"]/g, (ch) => ENTITY[ch as keyof typeof ENTITY]);
}
// odds as "1 in N" (N rounded from 100 up, one decimal below)
export const fmtIn = (p: number): string => {
  if (!(p > 0)) return 'never';
  const runs = 1 / p;
  return `1 in ${runs >= 100 ? Math.round(runs).toLocaleString() : runs.toFixed(1)}`;
};
// a click on the dialog's backdrop (the dialog element itself is the target) closes it
export const backdropCloses = (dialog: HTMLDialogElement) =>
  dialog.addEventListener('click', (e) => {
    if (e.target === dialog) dialog.close();
  });
