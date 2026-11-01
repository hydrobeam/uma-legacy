// Page-wide: an error or a rejected promise that nothing handled is logged to the console as an error, with what it
// carries. The browser suite fails a group on a console error (test/helpers.mjs), and Firefox does not report an
// unhandled rejection to it otherwise. The first import of both entries, so it is listening before any other module runs.
// The page shows nothing of it, except for an error before the entry's own code has run to its end (`started`, its last
// statement): that one stopped the entry part-way, before it drew anything (a module that throws as it loads, on data a
// build should have refused), so the status line says the page could not start instead of staying empty. Later errors
// (a view that fails to draw, reported by its RenderGuard) leave the status line to the page.
let starting = true;
/** the entry has run to its end: from now on its views and runs say what failed */
export const started = (): void => {
  starting = false;
};
addEventListener('error', (e) => {
  console.error('uncaught:', e.error ?? e.message);
  if (!starting) return;
  const note = document.getElementById('run-note'),
    head = document.getElementById('rhead');
  if (note) note.textContent = `The page could not start: ${e.error instanceof Error ? e.error.message : e.message}`;
  if (head) head.dataset.kind = 'error';
});
addEventListener('unhandledrejection', (e) => console.error('unhandled rejection:', e.reason));
