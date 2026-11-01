// The site in dist/ (./build.sh page): index.html (the career planner, the front page), loop.html (the loop planner)
// and about.html, from the pages here; their scripts (src/career/main.tsx, src/loop/main.tsx and the solver's worker,
// src/worker.ts), the stylesheet (src/styles/app.css: Tailwind's utilities and the theme, the fonts), the solver
// (loopopt.opt.wasm), the page data (vite/pagedata.mjs), the game's images (vite/images.mjs) and /about's math font
// (vite/mathfont.mjs). Every file in assets/ carries a fingerprint of its content in its name, so it is served with a
// one-year immutable cache and an update gets new names; the pages keep a 5-minute cache and always point at the
// current names (public/_headers). public/ (favicons, hosting rules) is copied to the root as it is. Served over
// http(s): the page fetches the wasm, data and images.
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import fs from 'node:fs';
import basicSsl from '@vitejs/plugin-basic-ssl';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';
import handlebars from 'vite-plugin-handlebars';
import { viteStaticCopy } from 'vite-plugin-static-copy';
import { imageCatalog, images } from './vite/images.mjs';
import { mathFont } from './vite/mathfont.mjs';
import { pageData, pageDataIds } from './vite/pagedata.mjs';
import { prerender } from './vite/prerender.mjs';
import { aboutFigures } from './vite/research.mjs';

const HERE = import.meta.dirname;
const IMG = path.join(HERE, 'img');

// the version in each page's footer: the git tag (and commits since it), "-dirty" with uncommitted changes to what the
// page is built from (webapp/, pkg/; edits elsewhere, e.g. docs, don't count)
function version() {
  const git = (...args) => execFileSync('git', args, { cwd: HERE, encoding: 'utf8' }).trim();
  try {
    const dirty = git('status', '--porcelain', '--untracked-files=no', '--', '../webapp', '../pkg');
    return git('describe', '--tags', '--always') + (dirty ? '-dirty' : '');
  } catch {
    return 'unknown';
  }
}
// the logo: the favicon's three sparkles inline in the title (gradient ids prefixed so they can't clash)
const logo = fs
  .readFileSync(path.join(HERE, 'public', 'favicon.svg'), 'utf8')
  .replace(/\bid="g(\d)"/g, 'id="lg$1"')
  .replaceAll('url(#g', 'url(#lg')
  .replace('<svg ', '<svg aria-hidden="true" focusable="false" ')
  .trim();

// what the site's build and the node tests (Vitest, mode 'test') share: the root, and JSX compiled to Preact's jsx()
// calls, imported from preact/jsx-runtime
const shared = {
  root: HERE,
  oxc: { jsx: { runtime: 'automatic', importSource: 'preact' } },
};
// the node tests (test/unit; ./build.sh lint, the pre-commit hook): the page's modules as the build compiles them, none
// of the build's plugins (its page data is each test's own, by vi.mock of '#page-data'); a test file that draws views
// asks for happy-dom's document (`@vitest-environment happy-dom`), at a desktop's size
const test = {
  include: ['test/unit/*.test.mjs'],
  environmentOptions: { happyDOM: { width: 1440, height: 900 } },
  testTimeout: 60_000,
  maxWorkers: 2, // at most two test files at once: the laptop's thermal limit, as the browser suite's two workers
};

export default defineConfig(({ mode }) => (mode === 'test' ? { ...shared, plugins: [pageDataIds()], test } : site()));

function site() {
  const catalog = imageCatalog(IMG);
  const VERSION = version();
  return {
    ...shared,
    appType: 'mpa', // /loop serves loop.html, as the host serves it; a path with no page is a 404
    // the version the planner pages' footer shows (drawn by their layouts: vite/prerender.mjs at build, the entry on load)
    define: { __LLP_VERSION__: JSON.stringify(VERSION) },
    plugins: [
      // the pages' shared head and header markup (partials/: the theme script, the header's tabs; /about's too) and the
      // values they show
      handlebars({
        partialDirectory: path.join(HERE, 'partials'),
        compileOptions: { strict: true }, // a value a page names and the build lacks stops the build
        helpers: { eq: (a, b) => a === b },
        context: {
          version: VERSION,
          // the site's address for the link-preview tags (og:url, og:image); a preview deploy passes its own
          site: (process.env.SITE_URL || 'https://uma-legacy.pages.dev').replace(/\/+$/, ''),
          logo,
          ...aboutFigures(),
        },
      }),
      // each planner page's one root, drawn into its HTML (the first paint), which its entry hydrates
      prerender({
        root: HERE,
        version: VERSION,
        pages: {
          'index.html': { module: 'src/career/Layout.tsx', layout: 'CareerLayout' },
          'loop.html': { module: 'src/loop/Layout.tsx', layout: 'LoopLayout' },
        },
      }),
      // (before Tailwind: the sprites' variables are appended to the stylesheet's entry before it compiles)
      images(catalog, IMG, path.join(HERE, 'src', 'styles', 'app.css')),
      // the utilities and the theme (src/styles/), compiled into the stylesheet
      tailwindcss(),
      pageData({ catalog, imgDir: IMG, wasm: '/loopopt.opt.wasm', solverPages: ['index.html', 'loop.html'] }),
      mathFont('about.html'),
      // `vite preview` (./build.sh serve, the browser suite) over HTTP/2 with TLS, as the host serves the site:
      // browsers speak HTTP/2 only over TLS, and over HTTP/1.1 a page's requests queue for six connections. The
      // certificate is self-signed, made once into cache/tls.
      basicSsl({ certDir: path.join(HERE, '..', 'cache', 'tls'), domains: ['127.0.0.1', 'localhost'] }),
      // the fonts' SIL Open Font Licenses, beside the fonts (not fingerprinted: kept out of assets/)
      viteStaticCopy({
        targets: ['zen-kaku-gothic-new', 'ibm-plex-mono', 'stix-two-math', 'zen-maru-gothic'].map((font) => ({
          src: `node_modules/@fontsource/${font}/LICENSE`,
          dest: 'licenses',
          rename: { stripBase: true, name: `OFL-${font}.txt` },
        })),
      }),
    ],
    build: {
      outDir: 'dist',
      emptyOutDir: true, // dist/ holds build output only: nothing stale is deployed
      assetsInlineLimit: 0, // every asset its own file, cached on its own
      rolldownOptions: {
        input: Object.fromEntries(['index', 'loop', 'about'].map((page) => [page, path.join(HERE, `${page}.html`)])),
        output: {
          // the modules both pages' scripts share, in one file
          chunkFileNames: 'assets/shared-[hash].js',
          // top-level const and let stay as written (Vite turns them into var by default): a read before its
          // declaration has run throws at load, which the browser suite sees, in place of an undefined read going on
          // unnoticed (Rolldown still writes a top-level class as `var X = class`)
          topLevelVar: false,
        },
      },
    },
  };
}
