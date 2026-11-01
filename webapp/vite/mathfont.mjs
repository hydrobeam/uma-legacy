// /about's math font: STIX Two Math subset by fontTools to the glyphs the page's formulas use (vite/mathfont.py), a
// fingerprinted file named by an @font-face in the page's own <style>, so only /about fetches it (and only to draw math).
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const REPO = path.join(import.meta.dirname, '..', '..');
const FACE = path.join(import.meta.dirname, '..', 'node_modules', '@fontsource', 'stix-two-math');
const NAME = 'stix-two-math.woff2';

/** The plugin, for the page `page` (its file name in the root). The page's source is what is subset: its formulas hold
 * no template value (vite.config.mjs fills only text outside them). */
export function mathFont(page) {
  return {
    name: 'llp-math-font',
    buildStart() {
      const subset = spawnSync(
        'uv',
        ['run', '--quiet', '--project', REPO, 'python', path.join(import.meta.dirname, 'mathfont.py')].concat([
          path.join(import.meta.dirname, '..', page),
          path.join(FACE, 'files', 'stix-two-math-latin-400-normal.woff2'),
        ]),
        { maxBuffer: 1 << 24 },
      );
      if (subset.status !== 0) throw new Error(`vite/mathfont.py failed: ${subset.stderr}`);
      this.emitFile({ type: 'asset', name: NAME, source: subset.stdout });
    },
    transformIndexHtml: {
      order: 'post',
      handler(html, { bundle, filename }) {
        if (path.basename(filename) !== page) return html;
        const font = Object.values(bundle).find((f) => f.type === 'asset' && f.names.includes(NAME));
        const face = fs.readFileSync(path.join(FACE, 'latin-400.css'), 'utf8');
        const css = face.replace(/src: [^;]+;/, `src: url(/${font.fileName}) format('woff2');`).trim();
        return { html, tags: [{ tag: 'style', children: `\n${css}\n`, injectTo: 'head' }] };
      },
    },
  };
}
