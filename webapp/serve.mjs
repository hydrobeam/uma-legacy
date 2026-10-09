// Minimal static server for dist/ (the page fetches loopopt.wasm and img/, so it is served, not opened as a file).
//   node webapp/serve.mjs [port]      serves webapp/dist on http://localhost:<port> (default 8000)
// Also imported by the browser suite (serveDist), which serves on a free port.
import http from 'http';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const DIST = path.join(path.dirname(fileURLToPath(import.meta.url)), 'dist');
const TYPES = { '.html': 'text/html; charset=utf-8', '.css': 'text/css; charset=utf-8', '.wasm': 'application/wasm', '.webp': 'image/webp', '.avif': 'image/avif', '.js': 'text/javascript', '.json': 'application/json', '.txt': 'text/plain', '.woff2': 'font/woff2', '.ico': 'image/x-icon', '.svg': 'image/svg+xml', '.png': 'image/png' };

export function serveDist(port = 0, host = '127.0.0.1') {
  const server = http.createServer((req, res) => {
    let p = decodeURIComponent(new URL(req.url, 'http://x').pathname);
    if (p.endsWith('/')) p += 'index.html';
    else if (!path.extname(p)) p += '.html';   // /about -> about.html, as Cloudflare Pages serves it
    const f = path.normalize(path.join(DIST, p));
    if (!f.startsWith(DIST + path.sep)) { res.writeHead(403); return res.end(); }
    fs.readFile(f, (err, buf) => {
      if (err) { res.writeHead(404); return res.end('not found'); }
      // as the host serves it (dist/_headers): fingerprinted assets/ a year, immutable (their names change with their
      // content); the pages and everything else revalidated on every load
      const cache = p.startsWith('/assets/') ? 'public, max-age=31536000, immutable' : 'no-cache';
      res.writeHead(200, { 'Content-Type': TYPES[path.extname(f)] || 'application/octet-stream', 'Cache-Control': cache });
      res.end(buf);
    });
  });
  return new Promise((resolve) => server.listen(port, host, () => resolve({ server, url: `http://${host}:${server.address().port}/` })));
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const { url } = await serveDist(+(process.argv[2] || 8000), 'localhost');
  console.log(`serving ${DIST} on ${url}  (Ctrl+C to stop)`);
}
