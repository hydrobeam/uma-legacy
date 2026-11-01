// Page-wide: an error or a rejected promise that nothing handled is logged to the console as an error, with what it
// carries. The browser suite fails a group on a console error (test/helpers.mjs), and Firefox does not report an
// unhandled rejection to it otherwise. The first import of both entries, so it is listening before any other module runs.
// The page shows nothing of it, except for an error while the page's scripts first run (the document still loading: the
// scripts sit at the end of the body, without defer): that one stopped the entry part-way, before it drew anything (a
// module that throws as it loads, on data a build should have refused), so the status line says the page could not
// start instead of staying empty.
addEventListener('error', (e) => {
  console.error('uncaught:', e.error ?? e.message);
  if (document.readyState !== 'loading') return;
  const note = document.getElementById('run-note'),
    head = document.getElementById('rhead');
  if (note) note.textContent = `The page could not start: ${e.error instanceof Error ? e.error.message : e.message}`;
  if (head) head.dataset.kind = 'error';
});
addEventListener('unhandledrejection', (e) => console.error('unhandled rejection:', e.reason));
