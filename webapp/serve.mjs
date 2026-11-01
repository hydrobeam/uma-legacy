// Minimal static server for dist/ (the page fetches loopopt.wasm and img/, so it is served, not opened as a file).
//   node webapp/serve.mjs [port] [host] [h2]   serves webapp/dist on http://<host>:<port> (default localhost:8000);
//   with h2, HTTP/2 over TLS (https://), as the host serves the site: browsers speak HTTP/2 only over TLS, and over
//   HTTP/1.1 a page's requests queue for six connections, so timings measured here would not be the site's. The
//   certificate is self-signed for 127.0.0.1 and localhost, made once by openssl into cache/tls/.
// The browser suite (playwright.config.mjs) starts it with h2 on a free port at 127.0.0.1.
import http from 'node:http';
import http2 from 'node:http2';
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const DIST = path.join(path.dirname(fileURLToPath(import.meta.url)), 'dist');
const TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.wasm': 'application/wasm',
  '.webp': 'image/webp',
  '.avif': 'image/avif',
  '.js': 'text/javascript',
  '.json': 'application/json',
  '.txt': 'text/plain',
  '.woff2': 'font/woff2',
  '.ico': 'image/x-icon',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
};

// cache/tls/{key,cert}.pem, made on first use
function selfSigned() {
  const dir = path.join(path.dirname(fileURLToPath(import.meta.url)), '..', 'cache', 'tls'),
    key = path.join(dir, 'key.pem'),
    cert = path.join(dir, 'cert.pem');
  if (!fs.existsSync(cert)) {
    fs.mkdirSync(dir, { recursive: true });
    execFileSync(
      'openssl',
      [
        ...['req', '-x509', '-newkey', 'ec', '-pkeyopt', 'ec_paramgen_curve:prime256v1', '-nodes'],
        ...['-keyout', key, '-out', cert, '-days', '3650', '-subj', '/CN=127.0.0.1'],
        ...['-addext', 'subjectAltName=IP:127.0.0.1,DNS:localhost'],
      ],
      { stdio: 'ignore' },
    );
  }
  return { key: fs.readFileSync(key), cert: fs.readFileSync(cert) };
}

function serveDist(port, host, h2 = false) {
  const handle = (req, res) => {
    let urlPath = decodeURIComponent(new URL(req.url, 'http://x').pathname);
    if (urlPath.endsWith('/')) urlPath += 'index.html';
    else if (!path.extname(urlPath)) urlPath += '.html'; // /about -> about.html, as Cloudflare Pages serves it
    const file = path.normalize(path.join(DIST, urlPath));
    if (!file.startsWith(DIST + path.sep)) {
      res.writeHead(403);
      return res.end();
    } // outside dist/ (a path with ..)
    fs.readFile(file, (err, buf) => {
      if (err) {
        res.writeHead(404);
        return res.end('not found');
      }
      // fingerprinted assets/ a year, immutable, as the host serves them (dist/_headers); the pages and everything else
      // revalidated on every load here (the host caches them 5 minutes), so a rebuild shows on the next load
      const cache = urlPath.startsWith('/assets/') ? 'public, max-age=31536000, immutable' : 'no-cache';
      res.writeHead(200, {
        'Content-Type': TYPES[path.extname(file)] || 'application/octet-stream',
        'Cache-Control': cache,
      });
      res.end(buf);
    });
  };
  const server = h2
    ? http2.createSecureServer({ ...selfSigned(), allowHTTP1: true }, handle)
    : http.createServer(handle);
  return new Promise((resolve) =>
    server.listen(port, host, () =>
      resolve({ server, url: `${h2 ? 'https' : 'http'}://${host}:${server.address().port}/` }),
    ),
  );
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const { url } = await serveDist(+(process.argv[2] || 8000), process.argv[3] || 'localhost', process.argv[4] === 'h2');
  console.log(`serving ${DIST} on ${url}  (Ctrl+C to stop)`);
}
