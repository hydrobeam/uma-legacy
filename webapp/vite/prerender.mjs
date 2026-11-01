// Each planner page's one root, prerendered: its layout (src/career/Layout.tsx, src/loop/Layout.tsx) drawn by Preact's
// renderToString into the page's `<div id="app"></div>`, with no views and no state, so the page's first paint has the
// setup's markup before any script runs; the entry hydrates it (main.tsx `hydrate`). The layout is imported through
// Vite's module runner (`runnerImport`: its TypeScript and JSX compiled as the page's are), on its own: a layout imports
// nothing but Preact and the shell (src/ui/Shell.tsx), so no store or game data loads here. (@preact/preset-vite's
// prerender, vite-prerender-plugin, renders every route into the first HTML input as one template: it can't fill two
// pages, each with its own head.)
import path from 'node:path';
import { h } from 'preact';
import { renderToString } from 'preact-render-to-string';
import { runnerImport } from 'vite';

const ROOT = '<div id="app"></div>';

/** `pages`: a page's file name -> its layout module (from the web root) and the layout's export; `version`: the footer's */
export function prerender({ root, pages, version }) {
  return {
    name: 'llp-prerender',
    transformIndexHtml: {
      order: 'pre',
      async handler(html, { filename }) {
        const page = pages[path.basename(filename)];
        if (!page) return html;
        if (html.split(ROOT).length !== 2) throw new Error(`${path.basename(filename)} needs one ${ROOT}`);
        const { module } = await runnerImport(path.join(root, page.module), {
          configFile: false,
          root,
          logLevel: 'error',
          oxc: { jsx: { runtime: 'automatic', importSource: 'preact' } },
        });
        const markup = renderToString(h(module[page.layout], { version }));
        return html.replace(ROOT, `<div id="app">${markup}</div>`);
      },
    },
  };
}
