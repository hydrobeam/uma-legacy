// An el()-built part rebuilt inside a component, a bridge until it is ported (frontend.md step 3): `build` empties and
// fills it again; the focus, if it was in `box`, goes back to the control of the same key (`data-k`), as the page's
// keepFocus does for the parts it redraws.
export function rebuild(box: HTMLElement, build: () => void) {
  const was = document.activeElement,
    k = was instanceof HTMLElement && box.contains(was) ? was.dataset.k : undefined;
  build();
  if (k) box.querySelector<HTMLElement>(`[data-k="${CSS.escape(k)}"]`)?.focus();
}
