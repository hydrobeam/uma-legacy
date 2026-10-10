// The runner's frames: webapp/chibi/chibi.html in headless Chromium (software WebGL), one uma after another.
// usage: node webapp/chibi/render.mjs WORK_DIR ID:SKIN:TAIL:BODY [...]
// Reads WORK_DIR/ID/{glb,t,run.json} (webapp/chibi/chibi.py writes them) and writes WORK_DIR/ID/frames/fNN.png: one
// loop of her run, 192 px square on a transparent background, from the game's three-quarter view.
import { createRequire } from 'module';
import http from 'http';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const H = path.dirname(fileURLToPath(import.meta.url));
process.env.PLAYWRIGHT_BROWSERS_PATH ??= '0';   // the browsers live in webapp/node_modules (./build.sh browsers)
let pw; try { pw = createRequire(import.meta.url)('playwright'); } catch { console.error('Playwright is missing: run ./build.sh browsers (or cd webapp && npm ci)'); process.exit(1); }
const { chromium } = pw;
const [work, ...jobs] = process.argv.slice(2);
const ROOTS = { '/three/': path.join(H, '..', 'node_modules', 'three'), '/work/': path.resolve(work) };
const TYPES = { '.html': 'text/html', '.js': 'text/javascript', '.json': 'application/json', '.png': 'image/png', '.glb': 'model/gltf-binary' };
const srv = http.createServer((q, s) => {
  const u = decodeURIComponent(new URL(q.url, 'http://x').pathname);
  const pre = Object.keys(ROOTS).find(p => u.startsWith(p));
  const f = u === '/chibi.html' ? path.join(H, 'chibi.html') : pre ? path.join(ROOTS[pre], u.slice(pre.length)) : null;
  if (!f || (pre && !f.startsWith(ROOTS[pre])) || !fs.existsSync(f)) { s.writeHead(404); return s.end(); }
  s.writeHead(200, { 'content-type': TYPES[path.extname(f)] || 'application/octet-stream' }); fs.createReadStream(f).pipe(s);
}).listen(0, '127.0.0.1');
await new Promise(r => srv.once('listening', r));
const port = srv.address().port;
const b = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
let failed = 0;
for (const job of jobs) {
  const [id, skin, tail, body = '00'] = job.split(':');
  const pg = await b.newPage(); const errs = [];
  pg.on('pageerror', e => errs.push(String(e))); pg.on('response', r => r.status() >= 400 && errs.push(r.status() + ' ' + r.url()));
  await pg.goto(`http://127.0.0.1:${port}/chibi.html?id=${id}&skin=${skin}&tail=${tail}&body=${body}&yaw=-40&pitch=6&size=${process.env.CHIBI_SIZE || 192}`);
  const ready = await pg.waitForFunction(() => window.ready || null, null, { timeout: 120000 }).then(h => h.jsonValue(), () => null);
  if (!ready || errs.length) { console.log(`${id}: failed ${errs.slice(0, 2).join(' | ')}`); failed++; await pg.close(); continue; }
  const dir = path.join(work, id, 'frames'); fs.mkdirSync(dir, { recursive: true });
  for (let i = 0; i < ready.frames; i++) {
    const d = await pg.evaluate((i) => window.frame(i), i);
    fs.writeFileSync(path.join(dir, `f${String(i).padStart(2, '0')}.png`), Buffer.from(d.split(',')[1], 'base64'));
  }
  console.log(`${id}: ${ready.frames} frames, tail ${ready.tail}, ${ready.springs} swaying bones`);
  await pg.close();
}
await b.close(); srv.close();
process.exit(failed ? 1 : 0);
