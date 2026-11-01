// Each page's first visit, brotli-compressed (quality 11, size-limit's), against its budget: ./build.sh page fails when
// a page goes over. A first visit fetches the page, every local file it names (its scripts, the stylesheet, the
// preloaded fonts, /about's math font) and, on the planner pages, the solver's worker, the solver and its Global data
// (the page starts the solver at once). Art is left out: it loads as seen. The host sends brotli at about quality 4,
// which reproduced its transfer to within 0.1% (measured against v50's deploy, 2026-10-31): these sizes are about 22%
// below what it sends on the planner pages (the wasm compresses further at 11) and 10% below on /about. A budget is
// raised in the commit that needs it, saying why.
const page = (name, files, limit) => ({ name, path: [`dist/${name}`, ...files.map((f) => `dist/assets/${f}`)], limit });
const everyPage = ['style-*.css', 'zen-kaku-gothic-new-latin-{400,700}-normal-*.woff2'];
// the planner pages': the scripts both share, and the solver
const planner = ['shared-*.js', 'worker-*.js', 'loopopt.opt-*.wasm', 'engine-global-*.json'];

export default [
  page('index.html', ['index-*.js', ...planner, ...everyPage], '888 kB'),
  page('loop.html', ['loop-*.js', ...planner, ...everyPage], '852 kB'),
  page('about.html', ['stix-two-math-*.woff2', ...everyPage], '111 kB'),
];
