// Render icons/favicon.svg to icons/_<n>.png (16, 32, 48, 180) in Chromium, for icons.py to pack. usage: node webapp/icons.mjs
import { createRequire } from 'module';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
const D = path.join(path.dirname(fileURLToPath(import.meta.url)), 'icons');
process.env.PLAYWRIGHT_BROWSERS_PATH ??= '0';   // the browsers live in webapp/node_modules (./build.sh browsers)
let pw; try { pw = createRequire(import.meta.url)('playwright'); } catch { console.error('Playwright is missing: run ./build.sh browsers (or cd webapp && npm ci)'); process.exit(1); }
const svg = fs.readFileSync(path.join(D, 'favicon.svg'), 'utf8');
const b = await pw.chromium.launch(); const pg = await b.newPage();
for (const n of [16, 32, 48, 180]) {
  await pg.setViewportSize({ width: n, height: n });
  await pg.setContent(`<html><body style="margin:0;background:transparent">${svg.replace('<svg ', `<svg width="${n}" height="${n}" `)}</body></html>`);
  await pg.screenshot({ path: path.join(D, `_${n}.png`), omitBackground: true, clip: { x: 0, y: 0, width: n, height: n } });
}
// the link-preview image: a 512x512 square of the sparkles on the page's background, shown as a compact thumbnail beside
// the text (twitter:card summary; Facebook and LinkedIn show images under 600 px wide as a small square too)
await pg.setViewportSize({ width: 512, height: 512 });
await pg.setContent(`<html><body style="margin:0;width:512px;height:512px;display:grid;place-items:center;background:#f3f5f8">${svg.replace('<svg ', '<svg width="400" height="400" ')}</body></html>`);
await pg.screenshot({ path: path.join(D, '_og.png'), clip: { x: 0, y: 0, width: 512, height: 512 } });
await b.close();
