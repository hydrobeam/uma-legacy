// Render icons/favicon.svg to icons/_<n>.png (16, 32, 48, 180) in Chromium, for icons.py to pack. usage: node webapp/icons.mjs
import { createRequire } from 'node:module';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const ICONS = path.join(path.dirname(fileURLToPath(import.meta.url)), 'icons');
process.env.PLAYWRIGHT_BROWSERS_PATH ??= '0'; // the browsers live in webapp/node_modules (./build.sh browsers)
let playwright;
try {
  playwright = createRequire(import.meta.url)('playwright');
} catch {
  console.error('Playwright is missing: run ./build.sh browsers (or cd webapp && npm ci)');
  process.exit(1);
}
const svg = fs.readFileSync(path.join(ICONS, 'favicon.svg'), 'utf8');
const browser = await playwright.chromium.launch();
const page = await browser.newPage();
for (const size of [16, 32, 48, 180]) {
  await page.setViewportSize({ width: size, height: size });
  await page.setContent(
    `<html><body style="margin:0;background:transparent">${svg.replace('<svg ', `<svg width="${size}" height="${size}" `)}</body></html>`,
  );
  await page.screenshot({
    path: path.join(ICONS, `_${size}.png`),
    omitBackground: true,
    clip: { x: 0, y: 0, width: size, height: size },
  });
}
// the link-preview image: a 512x512 square of the sparkles on the page's background, shown as a compact thumbnail beside
// the text (twitter:card summary; Facebook and LinkedIn show images under 600 px wide as a small square too)
await page.setViewportSize({ width: 512, height: 512 });
await page.setContent(
  `<html><body style="margin:0;width:512px;height:512px;display:grid;place-items:center;background:#f3f5f8">${svg.replace('<svg ', '<svg width="400" height="400" ')}</body></html>`,
);
await page.screenshot({ path: path.join(ICONS, '_og.png'), clip: { x: 0, y: 0, width: 512, height: 512 } });
await browser.close();
