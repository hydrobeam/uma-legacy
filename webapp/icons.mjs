// Render icons/favicon.svg to icons/_<n>.png (16, 32, 48, 180) in Chromium, for icons.py to pack. usage: node webapp/icons.mjs
import { createRequire } from 'module';
import { execSync } from 'child_process';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
const D = path.join(path.dirname(fileURLToPath(import.meta.url)), 'icons');
let pw; try { pw = createRequire(import.meta.url)('playwright'); } catch { pw = createRequire(execSync('npm root -g').toString().trim() + '/')('playwright'); }
const svg = fs.readFileSync(path.join(D, 'favicon.svg'), 'utf8');
const b = await pw.chromium.launch(); const pg = await b.newPage();
for (const n of [16, 32, 48, 180]) {
  await pg.setViewportSize({ width: n, height: n });
  await pg.setContent(`<html><body style="margin:0;background:transparent">${svg.replace('<svg ', `<svg width="${n}" height="${n}" `)}</body></html>`);
  await pg.screenshot({ path: path.join(D, `_${n}.png`), omitBackground: true, clip: { x: 0, y: 0, width: n, height: n } });
}
// the link-preview image (Open Graph, 1200x630): the sparkles centred on the page's background
await pg.setViewportSize({ width: 1200, height: 630 });
await pg.setContent(`<html><body style="margin:0;width:1200px;height:630px;display:grid;place-items:center;background:#f3f5f8">${svg.replace('<svg ', '<svg width="440" height="440" ')}</body></html>`);
await pg.screenshot({ path: path.join(D, '_og.png'), clip: { x: 0, y: 0, width: 1200, height: 630 } });
await b.close();
