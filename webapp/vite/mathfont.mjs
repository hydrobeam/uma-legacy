// /about's formulas (MathML) in STIX Two Math, subset by harfbuzz (subset-font) to what the page's <math> elements hold
// (the whole face is 403 KB; this, about 33 KB), its MATH table kept with the glyphs it reaches (stretched brackets,
// big operators). MathML draws a single-letter <mi> in math italic, so the italic Latin and Greek letters, ℎ and the
// invisible operators are kept too. A fingerprinted file named by an @font-face in the page's own <style>, so only
// /about fetches it (and only to draw math).
import fs from 'node:fs';
import path from 'node:path';
import { decodeHTML } from 'entities';
import subsetFont from 'subset-font';

const FACE = path.join(import.meta.dirname, '..', 'node_modules', '@fontsource', 'stix-two-math');
const NAME = 'stix-two-math.woff2';
const range = (from, to) => Array.from({ length: to - from }, (_, k) => from + k);
const MATH_EXTRA = [...range(0x1d434, 0x1d468), 0x210e, ...range(0x1d6e2, 0x1d71c), ...range(0x2061, 0x2065)];

/** The plugin, for the page `page` (its file name in the root). The page's source is what is subset: its formulas hold
 * no template value (vite.config.mjs fills only text outside them). */
export function mathFont(page) {
  return {
    name: 'llp-math-font',
    async buildStart() {
      const html = fs.readFileSync(path.join(import.meta.dirname, '..', page), 'utf8');
      const formulas = [...html.matchAll(/<math\b[\s\S]*?<\/math>/g)].map((m) =>
        decodeHTML(m[0].replace(/<[^>]+>/g, '')),
      );
      const face = fs.readFileSync(path.join(FACE, 'files', 'stix-two-math-latin-400-normal.woff2'));
      const text = formulas.join('') + String.fromCodePoint(...MATH_EXTRA);
      this.emitFile({ type: 'asset', name: NAME, source: await subsetFont(face, text, { targetFormat: 'woff2' }) });
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
