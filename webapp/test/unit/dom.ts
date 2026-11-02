// Lookups the node tests make in happy-dom's document, typed by checking what they find (a missing element or one of
// another kind throws, naming the selector), and the pause that lets Preact's effects and queued tasks run.
/** the element `selector` names under `root`, an HTMLElement (throws: none, or not an HTML element) */
export function find(root: ParentNode, selector: string): HTMLElement {
  const el = root.querySelector(selector);
  if (!(el instanceof HTMLElement)) throw new Error(`no HTML element ${selector}`);
  return el;
}
/** the element `selector` names, or null when there is none */
export function query(root: ParentNode, selector: string): HTMLElement | null {
  const el = root.querySelector(selector);
  if (el === null) return null;
  if (!(el instanceof HTMLElement)) throw new Error(`${selector} is not an HTML element`);
  return el;
}
/** the dialog `selector` names (throws: none, or not a dialog) */
export function dialogAt(root: ParentNode, selector: string): HTMLDialogElement {
  const el = root.querySelector(selector);
  if (!(el instanceof HTMLDialogElement)) throw new Error(`no dialog ${selector}`);
  return el;
}
/** a few rounds of queued tasks: what a render's effects and a dialog's close start has run */
export const settle = async (): Promise<void> => {
  for (let i = 0; i < 5; i++) await new Promise((r) => setTimeout(r, 0));
};
