// Page-wide: an error or a rejected promise that nothing handled is logged to the console as an error, with what it
// carries. The page shows nothing. The browser suite fails a group on a console error (test/helpers.mjs), and Firefox
// does not report an unhandled rejection to it otherwise. The first import of both entries, so it is listening before
// any other module runs.
addEventListener('error', (e) => console.error('uncaught:', e.error ?? e.message));
addEventListener('unhandledrejection', (e) => console.error('unhandled rejection:', e.reason));
