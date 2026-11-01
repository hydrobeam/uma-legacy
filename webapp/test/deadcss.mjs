// Dead CSS (./build.sh lint): the selectors of src/style.css that PurgeCSS would drop, a class, id, element or attribute
// in them named by none of the page's scripts, pages, partials and build plugins (as a word: a name in a comment counts).
// Exits 1 with the list; delete the dead rule, or list a name the code builds from parts in BUILT with where.
import { PurgeCSS } from 'purgecss';

const BUILT = {
  y0: 'ui/Agenda.tsx, the year columns: ag-year y + the year',
  y1: 'ui/Agenda.tsx, the year columns: ag-year y + the year',
  y2: 'ui/Agenda.tsx, the year columns: ag-year y + the year',
};
const [{ rejected }] = await new PurgeCSS().purge({
  content: ['src/**/*.{ts,tsx}', '*.html', 'partials/*.hbs', 'vite.config.mjs', 'vite/*.mjs'],
  css: ['src/style.css'],
  safelist: Object.keys(BUILT),
  rejected: true,
});
if (rejected.length) {
  console.log(`src/style.css: selectors nothing names (${rejected.length}):\n${rejected.join('\n')}`);
  process.exit(1);
}
