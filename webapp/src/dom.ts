// DOM shorthands the page modules share.
// (a literal id is on the page: test/static.mjs checks every one against the pages that run the module)
export const $ = (id: string): HTMLElement => document.getElementById(id) as HTMLElement;
// the same lookup for a check or text box: no check is made, the type only tells the type checker which properties it
// has
export const $input = (id: string): HTMLInputElement => document.getElementById(id) as HTMLInputElement;
// odds as "1 in N" (N rounded from 100 up, one decimal below)
export const fmtIn = (p: number): string => {
  if (!(p > 0)) return 'never';
  const runs = 1 / p;
  return `1 in ${runs >= 100 ? Math.round(runs).toLocaleString() : runs.toFixed(1)}`;
};
