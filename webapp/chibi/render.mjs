// The runner's frames: webapp/chibi/chibi.html in headless Chromium (software WebGL), one uma after another.
// usage: node webapp/chibi/render.mjs WORK_DIR ID:SKIN:TAIL:BODY [...]
// Reads WORK_DIR/ID/{glb,t,run.json} (webapp/chibi/chibi.py writes them) and writes WORK_DIR/ID/frames/fNN.png: one
// loop of her run, 192 px square on a transparent background, from the game's three-quarter view.
import { createRequire } from 'node:module';
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
process.env.PLAYWRIGHT_BROWSERS_PATH ??= '0'; // the browsers live in webapp/node_modules (./build.sh browsers)
let playwright;
try {
  playwright = createRequire(import.meta.url)('playwright');
} catch {
  console.error('Playwright is missing: run ./build.sh browsers (or cd webapp && npm ci)');
  process.exit(1);
}
const { chromium } = playwright;
const [work, ...jobs] = process.argv.slice(2);
// what the page loads: chibi.html, three.js from node_modules, and the work directory's models and textures
const ROOTS = { '/three/': path.join(HERE, '..', 'node_modules', 'three'), '/work/': path.resolve(work) };
const TYPES = {
  '.html': 'text/html',
  '.js': 'text/javascript',
  '.json': 'application/json',
  '.png': 'image/png',
  '.glb': 'model/gltf-binary',
};
const server = http
  .createServer((req, res) => {
    const urlPath = decodeURIComponent(new URL(req.url, 'http://x').pathname);
    const prefix = Object.keys(ROOTS).find((p) => urlPath.startsWith(p));
    const file =
      urlPath === '/chibi.html'
        ? path.join(HERE, 'chibi.html')
        : prefix
          ? path.join(ROOTS[prefix], urlPath.slice(prefix.length))
          : null;
    if (!file || (prefix && !file.startsWith(ROOTS[prefix])) || !fs.existsSync(file)) {
      res.writeHead(404);
      return res.end();
    }
    res.writeHead(200, { 'content-type': TYPES[path.extname(file)] || 'application/octet-stream' });
    fs.createReadStream(file).pipe(res);
  })
  .listen(0, '127.0.0.1');
await new Promise((resolve) => server.once('listening', resolve));
const port = server.address().port;
const browser = await chromium.launch({
  args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'],
});
let failed = 0;
for (const job of jobs) {
  const [id, skin, tail, body = '00'] = job.split(':');
  const page = await browser.newPage();
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  page.on('response', (r) => {
    if (r.status() >= 400) errors.push(`${r.status()} ${r.url()}`);
  });
  await page.goto(
    `http://127.0.0.1:${port}/chibi.html?id=${id}&skin=${skin}&tail=${tail}&body=${body}&yaw=-40&pitch=6&size=${process.env.CHIBI_SIZE || 192}`,
  );
  // window.ready: the page's report once the model is posed ({frames, tail, springs}); null when it never comes
  const ready = await page
    .waitForFunction(() => window.ready || null, null, { timeout: 120000 })
    .then(
      (h) => h.jsonValue(),
      () => null,
    );
  if (!ready || errors.length) {
    console.log(`${id}: failed ${errors.slice(0, 2).join(' | ')}`);
    failed++;
    await page.close();
    continue;
  }
  const dir = path.join(work, id, 'frames');
  fs.mkdirSync(dir, { recursive: true });
  for (let i = 0; i < ready.frames; i++) {
    const dataUrl = await page.evaluate((i) => window.frame(i), i); // a PNG data: URL
    fs.writeFileSync(
      path.join(dir, `f${String(i).padStart(2, '0')}.png`),
      Buffer.from(dataUrl.split(',')[1], 'base64'),
    );
  }
  console.log(`${id}: ${ready.frames} frames, tail ${ready.tail}, ${ready.springs} swaying bones`);
  await page.close();
}
await browser.close();
server.close();
process.exit(failed ? 1 : 0);
