import { PAGE_URL, check, open } from './helpers.mjs';

export default async function delivery(b) {
  // self-hosted fonts (no third-party requests), a real favicon, the solver's files and worker fingerprinted, the first
  // roster portraits eager, and accessible names that start with the visible text (voice control)
  const hosts = new Set();
  const urls = [];
  const pg = await open(b, {
    before: (p) =>
      p.on('request', (r) => {
        const u = new URL(r.url());
        if (u.protocol.startsWith('http')) {
          hosts.add(u.host);
          urls.push(u.pathname);
        }
      }),
  });
  check('no third-party requests (fonts self-hosted)', hosts.size === 1, [...hosts].join(' '));
  // (served as the host serves it, HTTP/2, so the page's requests don't queue as they would over HTTP/1.1; LLP_URL's
  // own server may differ)
  const protocols = await pg.evaluate(() => [
    ...new Set(performance.getEntriesByType('resource').map((e) => e.nextHopProtocol)),
  ]);
  check(
    'the page and its files come over HTTP/2',
    protocols.length > 0 && protocols.every((p) => p === 'h2'),
    protocols.join(' '),
  );
  check(
    'the solver data and binary come from fingerprinted files',
    urls.some((u) => /^\/assets\/engine-global-[\w-]{8}\.json$/.test(u)) &&
      urls.some((u) => /^\/assets\/loopopt\.opt-[\w-]{8}\.wasm$/.test(u)),
    urls.filter((u) => u.startsWith('/assets/') && !u.includes('/img.')).join(' '),
  );
  // (cached a year like them, not in the page, which is cached 5 minutes)
  // (Firefox reports the worker's URL as the page wrote it, relative)
  const workers = pg.workers().map((w) => new URL(w.url(), pg.url()).pathname);
  check(
    "the solver's worker script comes from a fingerprinted file",
    workers.length === 1 && /^\/assets\/worker-[\w-]{8}\.js$/.test(workers[0]),
    workers.join(' '),
  );
  check(
    'page fonts are the self-hosted subsets',
    await pg.evaluate(async () => {
      await document.fonts.ready;
      return [...document.fonts].some((f) => f.family.includes('Zen Kaku') && f.status === 'loaded');
    }),
  );
  const ico = await pg.request.get(new URL('favicon.ico', PAGE_URL).href);
  check(
    'favicon.ico is an icon, not the page',
    ico.ok() && /image\//.test(ico.headers()['content-type'] || ''),
    ico.headers()['content-type'],
  );
  check(
    'the first roster portraits load eagerly',
    await pg.evaluate(() => {
      const im = [...document.querySelectorAll('#roster [data-av] img')];
      return (
        im.length > 8 &&
        im.slice(0, 8).every((i) => i.loading === 'eager') &&
        im.slice(8).every((i) => i.loading === 'lazy')
      );
    }),
  );
  check(
    'sort buttons are named by their visible text first',
    await pg.evaluate(() =>
      ['rs', 'cs'].every((id) => {
        const e = document.getElementById(id);
        return e.getAttribute('aria-label').startsWith(e.textContent.trim());
      }),
    ),
  );
  check(
    'card tiles take their name from their visible text',
    await pg.evaluate(() =>
      [...document.querySelectorAll('#cards .ct')].every(
        (e) => !e.hasAttribute('aria-label') && e.hasAttribute('aria-pressed'),
      ),
    ),
  );
  await pg.context().close();
}
