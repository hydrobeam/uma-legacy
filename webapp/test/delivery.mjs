import { PAGE_URL, check, open } from './helpers.mjs';

export default async function delivery(b) {
  // self-hosted fonts (no third-party requests), a real favicon, the solver's files fingerprinted, the first roster
  // portraits eager, and accessible names that start with the visible text (voice control)
  const hosts = new Set(); const urls = [];
  const pg = await open(b, { before: (p) => p.on('request', r => { const u = new URL(r.url()); if (u.protocol.startsWith('http')) { hosts.add(u.host); urls.push(u.pathname); } }) });
  check('no third-party requests (fonts self-hosted)', hosts.size === 1, [...hosts].join(' '));
  check('the solver data and binary come from fingerprinted files', urls.some(u => /^\/assets\/engine-global\.[0-9a-f]{10}\.json$/.test(u)) && urls.some(u => /^\/assets\/loopopt\.[0-9a-f]{10}\.wasm$/.test(u)), urls.filter(u => u.startsWith('/assets/') && !u.includes('/img.')).join(' '));
  check('page fonts are the self-hosted subsets', await pg.evaluate(async () => { await document.fonts.ready; return [...document.fonts].some(f => f.family.includes('Zen Kaku') && f.status === 'loaded'); }));
  const ico = await pg.request.get(new URL('favicon.ico', PAGE_URL).href);
  check('favicon.ico is an icon, not the page', ico.ok() && /image\//.test(ico.headers()['content-type'] || ''), ico.headers()['content-type']);
  check('the first roster portraits load eagerly', await pg.evaluate(() => { const im = [...document.querySelectorAll('#roster .av img')]; return im.length > 8 && im.slice(0, 8).every(i => i.loading === 'eager') && im.slice(8).every(i => i.loading === 'lazy'); }));
  check('sort buttons are named by their visible text first', await pg.evaluate(() => ['rs', 'cs'].every(id => { const e = document.getElementById(id); return e.getAttribute('aria-label').startsWith(e.textContent.trim()); })));
  check('card tiles take their name from their visible text', await pg.evaluate(() => [...document.querySelectorAll('#cards .ct')].every(e => !e.hasAttribute('aria-label') && e.hasAttribute('aria-pressed'))));
  await pg.context().close();
}
