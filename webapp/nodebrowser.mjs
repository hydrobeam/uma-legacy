// Browser regression suite for the built page (webapp/dist, served on a local port; LLP_URL tests another address).
// Node + Playwright (local or global install). LLP_SHOTS=dir saves pictures of the scenario button and picker there.
//   node webapp/nodebrowser.mjs                 every test, Chromium and Firefox
//   node webapp/nodebrowser.mjs chromium roster only the named browsers / tests (any order)
// Both browsers run at once, each with LLP_JOBS test groups in parallel (default 4; LLP_JOBS=1 runs them one by one);
// a group's lines print together when it ends. Exit code 1 if any check fails. Expectations come from the page's own embedded data where possible, so a data
// update does not break the suite; the one fixed number is the Global default search (1 in 139, R cards off, Unity Cup).
import { createRequire } from 'module';
import { execSync } from 'child_process';
import { AsyncLocalStorage } from 'async_hooks';
import path from 'path';
import { fileURLToPath } from 'url';
import { serveDist } from './serve.mjs';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const local = process.env.LLP_URL ? null : await serveDist();
const URL = process.env.LLP_URL || local.url;
let pw;
try { pw = createRequire(import.meta.url)('playwright'); }
catch { pw = createRequire(execSync('npm root -g').toString().trim() + '/')('playwright'); }

const argv = process.argv.slice(2);
const BROWSERS = ['chromium', 'firefox'].filter(b => !argv.some(a => ['chromium', 'firefox'].includes(a)) || argv.includes(b));
let failed = 0, passed = 0;
const LOG = new AsyncLocalStorage();   // the running group's buffered lines (groups run in parallel)
const say = (line) => { const buf = LOG.getStore(); if (buf) buf.push(line); else console.log(line); };
const check = (name, ok, detail = '') => { if (ok) passed++; else failed++; say(`${ok ? '  ok  ' : '  FAIL'} ${name}${detail ? ' · ' + detail : ''}`); };

// ---------- helpers
async function open(browser, { w = 1440, h = 900, mobile = false, init } = {}) {
  const ctx = await browser.newContext({ viewport: { width: w, height: h }, ...(mobile && browser.browserType().name() === 'chromium' ? { isMobile: true, hasTouch: true } : {}) });
  await ctx.addInitScript(() => {   // count worker creations/terminations
    const W = window.Worker; window.__made = 0; window.__killed = 0;
    window.__runs = [];   // the jobs sent to the engine, in order
    window.Worker = function (...a) { const w = new W(...a); window.__made++; const t = w.terminate.bind(w); w.terminate = () => { window.__killed++; t(); };
      const pm = w.postMessage.bind(w); w.postMessage = (m, ...r) => { if (m && m.type === 'run') window.__runs.push(m.job); return pm(m, ...r); }; return w; };
  });
  if (init) await ctx.addInitScript(init);
  const pg = await ctx.newPage(); pg.errors = [];
  pg.on('pageerror', e => pg.errors.push(String(e))); pg.on('console', m => m.type() === 'error' && !/downloadable font/.test(m.text()) && pg.errors.push(m.text()));   // web-font fetch aborts are network noise
  await pg.goto(URL); await settle(pg);
  return pg;
}
const busy = (pg) => pg.evaluate(() => document.getElementById('run-note').classList.contains('busy'));
async function settle(pg, t = 180000) {   // wait for the current (auto) search to finish
  await pg.waitForTimeout(600);
  await pg.waitForFunction(() => { const n = document.getElementById('run-note'); return !n.classList.contains('busy') && /Searched|example/i.test(n.textContent); }, null, { timeout: t });
}
const note = (pg) => pg.locator('#run-note').innerText();
const odds = (pg) => pg.evaluate(() => (document.querySelector('#results .loop .odds') || {}).textContent || '');
const json = (pg, id) => pg.evaluate((id) => JSON.parse(document.getElementById(id).textContent), id);
const scen = (pg) => pg.getAttribute('#x-scen', 'data-value');   // the chosen scenario
async function pickScen(pg, v) { await pg.click('#x-scen'); await pg.click(`#scen-menu .stile[data-v="${v}"]`); }

// WCAG 2 contrast of every visible text run in the open dialog (or the page)
const AUDIT = () => {
  const parse = (c) => { const m = c.match(/rgba?\(([^)]+)\)/); if (!m) return null; const p = m[1].split(/[ ,/]+/).filter(Boolean).map(Number); return { r: p[0], g: p[1], b: p[2], a: p.length > 3 ? p[3] : 1 }; };
  const over = (f, b) => ({ r: f.r * f.a + b.r * (1 - f.a), g: f.g * f.a + b.g * (1 - f.a), b: f.b * f.a + b.b * (1 - f.a), a: 1 });
  const lum = (c) => { const ch = (v) => { v /= 255; return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; }; return 0.2126 * ch(c.r) + 0.7152 * ch(c.g) + 0.0722 * ch(c.b); };
  const ratio = (a, b) => { const x = lum(a), y = lum(b); return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05); };
  const bgOf = (el) => { const layers = []; let e = el, img = false;
    for (; e; e = e.parentElement) { const cs = getComputedStyle(e); if (cs.backgroundImage !== 'none' && !cs.backgroundImage.startsWith('linear')) img = true;
      const c = parse(cs.backgroundColor); if (c && c.a > 0) { layers.push(c); if (c.a >= 1) break; } }
    let bg = { r: 255, g: 255, b: 255, a: 1 }; for (let i = layers.length - 1; i >= 0; i--) bg = over(layers[i], bg); return { bg, img }; };
  const opac = (el) => { let o = 1; for (let e = el; e; e = e.parentElement) o *= +getComputedStyle(e).opacity; return o; };
  const out = []; const dlg = [...document.querySelectorAll('dialog[open]')]; const scope = dlg.length ? dlg[dlg.length - 1] : document.body;
  for (const el of scope.querySelectorAll('*')) {
    const txt = [...el.childNodes].filter(n => n.nodeType === 3).map(n => n.textContent).join('').trim(); if (!txt) continue;
    const r = el.getBoundingClientRect(); if (!r.width || !r.height) continue;
    const cs = getComputedStyle(el); if (cs.visibility === 'hidden' || cs.display === 'none' || el.closest('[hidden]') || el.closest('select')) continue;
    const { bg, img } = bgOf(el); if (img) continue;
    const fg0 = parse(cs.color); const fg = over({ ...fg0, a: fg0.a * opac(el) }, bg);
    const px = parseFloat(cs.fontSize), large = px >= 24 || (+cs.fontWeight >= 700 && px >= 18.66), need = large ? 4.5 : 7;   // AAA
    const rt = ratio(fg, bg); if (rt < need) out.push(`${el.tagName.toLowerCase()}.${el.className} "${txt.slice(0, 30)}" ${rt.toFixed(2)}<${need}`);
  }
  return out;
};

// ---------- tests: each gets a fresh page (fresh storage) unless noted
const TESTS = {
  async load(b) {
    const pg = await open(b);
    check('first search runs by itself', /Searched/.test(await note(pg)), await note(pg));
    check('Global default search: 1 in 139', /1 in 139/.test(await odds(pg)), await odds(pg));
    // the deck's odds per target: each with its skill icon, the white named (no "any hint"), no lead text
    const dd = await pg.evaluate(() => { const l = document.querySelector('#results .loop .dodds'); return { n: l.querySelectorAll('.dsk').length, icons: l.querySelectorAll('.dsk img.sic').length, text: l.innerText }; });
    check('deck odds: a skill icon on every entry, no "any hint" or lead text', dd.n > 0 && dd.icons === dd.n && !/any hint|every run/i.test(dd.text), JSON.stringify(dd));
    // the odds sit in a frame the (i) button opens; a swap row names its uma by her character icon
    const oi = pg.locator('#results .loop .deck .oinfo').first();
    check('deck odds start closed', await pg.evaluate(() => document.querySelector('#results .loop .dodds').hidden));
    await oi.click();
    check('(i) opens the deck odds (aria-expanded, names and percentages)', (await oi.getAttribute('aria-expanded')) === 'true' && await pg.evaluate(() => { const o = document.querySelector('#results .loop .dodds'); return !o.hidden && o.querySelectorAll('.dnm').length > 0 && /\d+%/.test(o.innerText); }));
    await oi.click();
    check('(i) closes them again', await pg.evaluate(() => document.querySelector('#results .loop .dodds').hidden));
    const sw = await pg.evaluate(() => [...document.querySelectorAll('#results .loop .deck.swap .swho')].map(w => ({ av: w.querySelectorAll('.av').length, label: w.querySelector('.vh').textContent, tips: [...w.querySelectorAll('.av')].map(a => a.title), shown: [...w.childNodes].filter(n => !(n.classList && n.classList.contains('vh'))).map(n => n.textContent).join('').trim() })));   // .vh: screen-reader text
    check('swap rows show the uma as a character icon, named for screen readers', sw.length > 0 && sw.every(x => x.av > 0 && (x.av === 1 ? /deck/ : / and /).test(x.label) && x.tips.length === x.av && x.tips.every(t => t && !/ and |’s deck/.test(t)) && x.shown === ''), JSON.stringify(sw));
    const off = await pg.evaluate(() => Math.round(document.getElementById('rcol').getBoundingClientRect().top - document.querySelector('.rail .panel').getBoundingClientRect().top));
    check('results start level with the settings column', Math.abs(off) <= 1, off + ' px');
    // the results stay in view while the settings column scrolls (sticky column, scrolling on its own)
    await pg.evaluate(() => scrollTo(0, 1600)); await pg.waitForTimeout(100);
    const st = await pg.evaluate(() => { const c = document.getElementById('rcol').getBoundingClientRect(), l = document.querySelector('#results .loop').getBoundingClientRect(), n = document.getElementById('run-note').getBoundingClientRect();
      return { top: Math.round(c.top), bottom: Math.round(c.bottom), loop: Math.round(l.top), note: Math.round(n.top), y: scrollY, h: innerHeight }; });
    check('desktop: scrolled down the settings, the status line and the result stay on screen', st.y > 1000 && st.top >= 0 && st.top <= 20 && st.note >= 0 && st.loop < st.h && st.bottom <= st.h, JSON.stringify(st));
    await pg.evaluate(() => scrollTo(0, 0));
    check('"Results to show" sits in the results header', await pg.evaluate(() => !!document.getElementById('c-top').closest('#rhead')));
    check('no button; status bar present', (await pg.locator('#run').count()) === 0 && (await pg.locator('#run-note').count()) === 1);
    // the scenario sets the training-hint rate: switching it searches again with the scenario in the job, and the
    // choice is kept (the default setup's odds barely move: its decks get the targets from events, not training hints)
    const n0 = await pg.evaluate(() => window.__runs.length);
    check('the first search is a Unity Cup job', await pg.evaluate(() => window.__runs.at(-1).scenario === 'unity'));
    await pickScen(pg, 'ura'); await settle(pg);
    check('scenario switch searches again with the URA job', await pg.evaluate((n) => window.__runs.length > n && window.__runs.at(-1).scenario === 'ura', n0));
    // Unity Cup's golds follow the scenario: off under URA, back on under Unity, and the 8/8 example takes them too
    check('URA drops the Unity golds', (await pg.getAttribute('#g-20049', 'data-g')) === '0' && (await pg.getAttribute('#g-20046', 'data-g')) === '0');
    await pg.click('#t-preset');
    check('8/8 example under URA: eight targets, no guaranteed golds', (await pg.locator('#targets .tgt').count()) === 8 && (await pg.getAttribute('#g-20049', 'data-g')) === '0');
    await settle(pg);
    await pg.reload(); await settle(pg);
    check('scenario survives a reload', (await scen(pg)) === 'ura' && (await pg.getAttribute('#x-scen', 'aria-label')) === 'Scenario: URA Finale');
    await pickScen(pg, 'unity');
    check('Unity Cup marks No Stopping Me! and It\'s On! as guaranteed golds', (await pg.getAttribute('#g-20049', 'data-g')) === '2' && (await pg.getAttribute('#g-20046', 'data-g')) === '2');
    await settle(pg);
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async scenario(b) {
    // the scenario picker: a button showing the chosen scenario's logo, opening a grid of the four
    const pg = await open(b);
    const menu = pg.locator('#scen-menu'), tiles = pg.locator('#scen-menu .stile');
    const btnName = () => pg.getAttribute('#x-scen', 'aria-label');
    check('picker closed at first; the button is Unity Cup (logo only, named for assistive tech)', (await menu.isHidden()) && (await scen(pg)) === 'unity' && (await btnName()) === 'Scenario: Unity Cup'
      && (await pg.evaluate(() => JSON.parse(document.getElementById('data-img').textContent).keys.includes('scen:2'))
        ? (await pg.locator('#x-scen').innerText()).trim() === '' && (await pg.locator('#x-scen img').count()) === 1
        : (await pg.locator('#x-scen').innerText()).trim() === 'Unity Cup'));
    const shots = process.env.LLP_SHOTS, bn = b.browserType().name();   // pictures of the button and the open picker, for a look at the logos
    if (shots) await pg.locator('#x-scen').screenshot({ path: `${shots}/${bn}-scen-button.png` });
    await pg.click('#x-scen');
    const t = await pg.evaluate(() => [...document.querySelectorAll('#scen-menu .stile')].map(t => { const r = t.getBoundingClientRect();
      const sub = t.querySelector('.ssub'), sr = sub.getBoundingClientRect(), pad = parseFloat(getComputedStyle(t).paddingBottom) + parseFloat(getComputedStyle(t).borderBottomWidth);
      return { v: t.dataset.v, sub: sub.innerText.replace(/\s+/g, ' ').trim(), text: t.innerText, w: Math.round(r.width), h: Math.round(r.height), gap: Math.round(r.bottom - pad - sr.bottom), subH: Math.round(sr.height) }; }));
    // hint rate as a multiple of URA Finale's (2.70, 2.08, 1.60, 2.30 hints per card over 1.60)
    const want = { unity: 'Hint rate ×1.69', gc: 'Hint rate ×1.30', ura: 'Hint rate ×1.00', trackblazer: 'Hint rate ×1.44 (estimate)' };
    check('opens with four tiles, each with its hint rate against URA Finale', !(await menu.isHidden()) && t.length === 4 && t.every(x => x.sub === want[x.v]), t.map(x => x.v + ': ' + x.sub).join(' | '));
    check('the four tiles are the same size', new Set(t.map(x => x.w + 'x' + x.h)).size === 1, t.map(x => x.w + 'x' + x.h).join(' '));
    check('every tile is logo + one line, no blank space below', t.every(x => x.subH <= 22 && x.gap <= 1), t.map(x => `${x.v} line ${x.subH}px, blank ${x.gap}px`).join(' | '));
    const NAMES = { unity: 'Unity Cup', gc: 'Grand Concert', ura: 'URA Finale', trackblazer: 'Trackblazer' };
    check('no scenario name under the logos (the logo is the name)', (await pg.locator('#scen-menu .stile img').count()) !== 4 || t.every(x => !x.text.includes(NAMES[x.v])), t.map(x => JSON.stringify(x.text)).join(' '));
    // Unity Cup's guaranteed golds: their gold skill icons on the hint-rate line, named by tooltip and alt text
    const golds = await pg.evaluate(() => { const skl = JSON.parse(document.getElementById('data-skills').textContent), gi = JSON.parse(document.getElementById('data-skillicon').textContent).gold;
      const g = document.querySelector('#scen-menu .stile[data-v="unity"] .ssub .sgolds');
      return { want: [20049, 20046].map(x => ({ name: skl.find(s => s[0] === x)[2], src: `/skill/${gi[x]}.avif` })), group: g && g.getAttribute('aria-label'),
        got: g ? [...g.querySelectorAll('img')].map(i => ({ alt: i.alt, title: i.title, src: i.getAttribute('src'), h: Math.round(i.getBoundingClientRect().height) })) : [],
        text: g ? g.innerText.trim() : null, others: document.querySelectorAll('#scen-menu .stile:not([data-v="unity"]) .sgolds').length }; });
    check('Unity Cup tile: No Stopping Me! and It\'s On! as gold icons on the rate line, no visible names', golds.group === 'Guaranteed in every run' && golds.others === 0 && golds.text === ''
      && golds.got.length === 2 && golds.want.every((w, i) => golds.got[i].alt === w.name && golds.got[i].title === w.name && golds.got[i].src.endsWith(w.src) && golds.got[i].h <= 18), JSON.stringify(golds.got));
    check('the word "golds" appears nowhere on the page', !/\bgolds\b/i.test(await pg.evaluate(() => document.body.innerText)));
    check('logos keep their aspect (object-fit: contain)', await pg.evaluate(() => [...document.querySelectorAll('#scen-dd .slogo img')].every(i => i.naturalWidth > 0 && getComputedStyle(i).objectFit === 'contain')));
    check('the open picker marks the chosen tile and focuses it', await pg.evaluate(() => { const t = document.activeElement; return t.classList.contains('stile') && t.dataset.v === 'unity' && t.getAttribute('aria-checked') === 'true'; })
      && (await pg.getAttribute('#x-scen', 'aria-expanded')) === 'true');
    if (shots) await menu.screenshot({ path: `${shots}/${bn}-scen-picker.png` });
    const r0 = await pg.evaluate(() => window.__runs.length);
    await tiles.nth(0).click(); await pg.waitForTimeout(400);
    check('re-picking the chosen scenario closes the picker and does not search', (await menu.isHidden()) && (await pg.evaluate(() => window.__runs.length)) === r0 && !/Searching/.test(await note(pg)));
    await pg.keyboard.press('ArrowDown');   // focus is back on the button
    check('arrow key on the button opens the picker', !(await menu.isHidden()));
    await pg.keyboard.press('ArrowRight'); await pg.keyboard.press('ArrowDown');
    check('arrows move between tiles (two columns)', await pg.evaluate(() => document.activeElement.dataset.v) === 'trackblazer');
    await pg.keyboard.press('Escape');
    check('Escape closes it and returns focus to the button', (await menu.isHidden()) && await pg.evaluate(() => document.activeElement.id === 'x-scen'));
    await pg.click('#x-scen'); await pg.click('header.top h1');
    check('a click outside closes it', await menu.isHidden());
    // only the button and the grid count as inside: the label, the row's empty space and the rank control all close it
    const shut = [];
    await pg.click('#x-scen'); await pg.click('#x-scen-l'); shut.push(await menu.isHidden());
    await pg.click('#x-scen'); const bb = await pg.locator('#x-scen').boundingBox(); await pg.mouse.click(bb.x + bb.width + 8, bb.y + bb.height / 2); shut.push(await menu.isHidden());
    await pg.click('#x-scen'); await pg.click('.rankctl > span'); shut.push(await menu.isHidden());
    check('clicks on the label, beside the button, on the rank control close it', shut.every(Boolean), shut.join(','));
    await pg.click('#x-scen'); await pg.keyboard.press('End'); await pg.keyboard.press('Enter'); await settle(pg);
    check('Enter on a tile picks it and closes the picker', (await menu.isHidden()) && (await scen(pg)) === 'trackblazer' && (await btnName()) === 'Scenario: Trackblazer');
    check('… and searches with it', await pg.evaluate(() => window.__runs.at(-1).scenario === 'trackblazer'));
    // the lineage rank sits in the Game data panel and follows the scenario: Grand Concert SS, Unity Cup below SS
    check('the lineage rank control is in the Game data panel', await pg.evaluate(() => !!document.getElementById('rank').closest('section[aria-labelledby="h-era"]')));
    await pickScen(pg, 'gc'); await settle(pg);
    check('picking Grand Concert sets the rank to SS', (await pg.getAttribute('#rank', 'data-value')) === '0.20,0.70,0.10' && await pg.evaluate(() => window.__runs.at(-1).star_dist.join() === '0.2,0.7,0.1'));
    await pickScen(pg, 'unity'); await settle(pg);
    check('picking Unity Cup sets it back to below SS', (await pg.getAttribute('#rank', 'data-value')) === '0.50,0.45,0.05');
    await pg.click('#rank button[data-v="0.20,0.70,0.10"]'); await settle(pg); await pickScen(pg, 'unity'); await pg.waitForTimeout(400);
    check('a rank changed by hand sticks (re-picking the scenario leaves it)', (await pg.getAttribute('#rank', 'data-value')) === '0.20,0.70,0.10' && await pg.evaluate(() => window.__runs.at(-1).star_dist.join() === '0.2,0.7,0.1'));
    await pg.reload(); await settle(pg);
    check('… and survives a reload', (await pg.getAttribute('#rank', 'data-value')) === '0.20,0.70,0.10' && (await scen(pg)) === 'unity');
    // the rank toggle is a radio group: game badges, arrow keys move the choice
    await pg.focus('#rank button[aria-checked="true"]'); await pg.keyboard.press('ArrowLeft'); await pg.waitForTimeout(200);
    check('lineage rank: arrow keys switch it, aria-checked follows, badges shown', (await pg.getAttribute('#rank', 'data-value')) === '0.50,0.45,0.05'
      && (await pg.getAttribute('#rank button[data-v="0.50,0.45,0.05"]', 'aria-checked')) === 'true' && (await pg.locator('#rank .rk.img img').count()) === 3);
    await pg.keyboard.press('ArrowRight'); await settle(pg);
    await pickScen(pg, 'trackblazer'); await settle(pg);
    await pg.click('#ex-open');
    check('export names the scenario', JSON.parse(await pg.inputValue('#ex-text')).readable.scenario === 'Trackblazer');
    const text = (await pg.inputValue('#ex-text')).replace('"scen": "trackblazer"', '"scen": "gc"');
    await pg.fill('#ex-text', text); await pg.click('#ex-load'); await pg.click('#ex-close');
    check('import updates the button', (await scen(pg)) === 'gc' && (await btnName()) === 'Scenario: Grand Concert');
    await settle(pg);
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
    // without the logo files (images not built, or a failed fetch) the button and tiles show the name on a badge
    const p2 = await b.newContext().then(c => c.newPage());
    await p2.route('**/img.*/scen/**', r => r.abort()); await p2.goto(URL); await p2.waitForTimeout(800); await p2.click('#x-scen');
    const badges = await p2.evaluate(() => [...document.querySelectorAll('#scen-dd .slogo')].map(l => (l.querySelector('img') ? 'img' : '') + (l.querySelector('b') || {}).textContent).join(','));
    check('missing logos fall back to name badges', badges === 'Unity Cup,Unity Cup,Grand Concert,URA Finale,Trackblazer', badges);
    const sz = await p2.evaluate(() => new Set([...document.querySelectorAll('#scen-menu .stile')].map(t => { const r = t.getBoundingClientRect(); return Math.round(r.width) + 'x' + Math.round(r.height); })).size);
    check('… and the tiles stay the same size', sz === 1);
    check('Grand Masters is not offered with the Global data', (await p2.locator('#scen-menu .stile').count()) === 4 && (await p2.locator('#scen-menu .stile[data-v="grandmasters"]').count()) === 0);
    check('… and the Unity Cup golds fall back to their names', (await p2.locator('#scen-menu .stile[data-v="unity"] .sgold').count()) === 2 || (await p2.locator('#scen-menu .sgolds img').count()) === 2);
    await p2.context().close();
  },
  async autosearch(b) {
    const pg = await open(b);
    const made0 = await pg.evaluate(() => window.__made);
    for (let i = 0; i < 5; i++) await pg.locator('#cards .ct').nth(i).click();
    check('a change shows "Searching…"', /Searching/.test(await note(pg)));
    await settle(pg);
    check('a burst of 5 changes is one search on the same worker', (await pg.evaluate(() => window.__made)) === made0);
    // a slow search (3 results, one shared deck), changed mid-run: the worker is killed and the new search finishes
    const r0 = await pg.evaluate(() => window.__runs.length);
    await pg.selectOption('#c-top', '3'); await pg.click('#c-peruma');
    await pg.waitForFunction((n) => window.__runs.length > n, r0); await pg.waitForTimeout(100);   // the slow search is running
    const k0 = await pg.evaluate(() => window.__killed);
    check('old results stay visible while searching', (await pg.locator('#results .loop').count()) > 0);
    await pg.click('#c-peruma'); await pg.selectOption('#c-top', '1'); await settle(pg);
    check('a change mid-search restarts the worker', (await pg.evaluate(() => window.__killed)) > k0);
    check('the restarted search finishes', /Searched/.test(await note(pg)));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async roster(b) {
    const pg = await open(b);
    const umas = await json(pg, 'data-umas'); const nOut = umas.reduce((a, u) => a + Math.max(1, u.outfits.length), 0);
    check('one tile per outfit', (await pg.locator('#roster .ut').count()) === nOut, nOut + ' outfits');
    const all = umas.flatMap(u => (u.outfits.length ? u.outfits : [{ label: '', rel: 0, id: 0 }]).map(o => ({ n: u.name + (o.label ? ' ' + o.label : ''), rel: o.rel, id: o.id })));
    const firstTile = async () => (await pg.locator('#roster .ut').first().innerText()).replace(/\n/g, ' ').trim();
    const newest = [...all].sort((a, c) => (c.rel - a.rel) || (c.id - a.id))[0].n, oldest = [...all].sort((a, c) => (a.rel - c.rel) || (a.id - c.id))[0].n;
    check('default sort: release, newest first', (await pg.locator('#rs').innerText()) === 'Release ↓' && await firstTile() === newest, await firstTile());
    await pg.click('#rs'); await pg.click('#rs-menu [data-k="rel"]');
    check('re-picking Release reverses it (oldest first)', (await pg.locator('#rs').innerText()) === 'Release ↑' && await firstTile() === oldest, await firstTile());
    check('the sort menu stays open after a pick', !(await pg.locator('#rs-menu').isHidden()));
    await pg.click('#rs-menu [data-k="name"]');
    const byName = [...all].sort((a, c) => a.n.localeCompare(c.n))[0].n;
    check('Name sorts A to Z', (await pg.locator('#rs').innerText()) === 'Name ↑' && await firstTile() === byName, await firstTile());
    await pg.keyboard.press('Escape'); check('Escape closes the menu', await pg.locator('#rs-menu').isHidden());
    // cycle an uma with several outfits: one outfit out, then must include (that outfit only), then owned
    const multi = umas.find(u => u.outfits.length > 1);
    const states = () => pg.evaluate((id) => [...document.querySelectorAll(`#roster .ut[data-uma="${id}"]`)].map(t => t.dataset.s).join(','), multi.id);
    const tile = pg.locator(`#roster .ut[data-uma="${multi.id}"] .up`).first();
    await tile.click(); const s1 = await states(); await tile.click(); const s2 = await states(); await tile.click(); const s3 = await states();
    check('tile cycle: that outfit not owned', s1.split(',').filter(x => x === 'out').length === 1, s1);
    check('tile cycle: must include marks only that outfit', s2.split(',').filter(x => x === 'req').length === 1 && s2.split(',').filter(x => x === 'own').length === s2.split(',').length - 1, s2);
    check('tile cycle: back to owned', s3.split(',').every(x => x === 'own'), s3);
    check('greyed, never crossed out', !(await pg.evaluate(() => [...document.querySelectorAll('.ut .un, .ut .uol, .ct .cn')].some(e => getComputedStyle(e).textDecorationLine.includes('line-through')))));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async cards(b) {
    const pg = await open(b);
    const secs = await pg.evaluate(() => [...document.querySelectorAll('#cards .cgroup h3')].map(h => h.textContent));
    check('card sections SSR, SR, R in that order', secs.join(',') === 'SSR,SR,R', secs.join(','));
    // the game bans Team Sirius from Unity Cup and Grand Concert decks: the picker says so and no result deck holds it
    const sirius = () => pg.evaluate(() => { const t = [...document.querySelectorAll('#cards .ct')].find(x => /Team Sirius/.test(x.textContent)); return t ? (t.querySelector('.cban') || {}).textContent || '' : null; });
    check('Team Sirius: "Not allowed in Unity Cup" in the card picker', (await sirius()) === 'Not allowed in Unity Cup', String(await sirius()));
    check('… and in no Unity Cup result deck', await pg.evaluate(() => ![...document.querySelectorAll('#results .dc')].some(d => d.dataset.cid === '30081')));
    await pickScen(pg, 'ura'); await settle(pg);
    check('Team Sirius allowed again in URA Finale', (await sirius()) === '');
    await pickScen(pg, 'unity'); await settle(pg);
    const sep = await pg.evaluate(() => { const c = document.getElementById('cards'), r = c.previousElementSibling.getBoundingClientRect(), cs = getComputedStyle(c);
      return { gap: Math.round(c.getBoundingClientRect().top - r.bottom), pad: parseFloat(cs.paddingTop) }; });
    check('the card list scrolls apart from its search row (gap like the roster\'s), its tiles\' rings and focus outline inside it', sep.gap >= 10 && sep.pad >= 5, JSON.stringify(sep));
    check('default card sort: release, newest first', (await pg.locator('#cs').innerText()) === 'Release ↓');
    const tile = pg.locator('#cards .ct').first(); const img = await tile.evaluate(t => { t.__img = t.querySelector('img'); return !!t.__img; });
    await tile.click();
    check('toggling a card keeps its image element (no flicker)', img && await tile.evaluate(t => t.querySelector('img') === t.__img && t.getAttribute('aria-pressed') === 'false'));
    const cnt = /^(\d+) of (\d+) listed cards owned$/.exec(await pg.locator('#c-count').innerText());
    check('the count line follows ("N of M listed cards owned")', !!cnt && +cnt[1] === +cnt[2] - 1, await pg.locator('#c-count').innerText());
    await tile.click();
    // R cards are off by default: the R section holds only the R pals until they are switched on
    const rpals = await pg.evaluate(() => JSON.parse(document.getElementById('data-cards').textContent).filter(c => c.rar === 1 && c.pal).length);
    const rCount = () => pg.evaluate(() => { const g = [...document.querySelectorAll('#cards .cgroup')].find(x => x.querySelector('h3').textContent === 'R'); return g ? g.querySelectorAll('.ct').length : 0; });
    check('R cards off by default; the R section keeps the R pals', !(await pg.locator('#x-rcards').isChecked()) && (await rCount()) === rpals, (await rCount()) + ' of ' + rpals);
    await pg.click('#x-rcards');
    check('switching R cards on lists them', (await rCount()) > rpals);
    check('every card image carries its type icon', (await pg.locator('#cards .ct .cav.art').count()) === (await pg.locator('#cards .ct .ctype').count()));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async panels(b) {
    const pg = await open(b);
    const dc = pg.locator('.deck .dc').first(); await dc.click(); await pg.waitForTimeout(100);
    const href = await pg.locator('.cpop .gtl').getAttribute('href');
    check('deck card panel: GameTora card link', /^https:\/\/gametora\.com\/umamusume\/supports\/\d+-[a-z0-9-]+$/.test(href || ''), href);
    const cid = await dc.getAttribute('data-cid');
    await pg.locator('.cpop .sw').click(); await pg.waitForTimeout(100);
    check('deck card Owned switch updates the card picker', await pg.evaluate((id) => [...document.querySelectorAll('#cards .ct')].some(t => t.querySelector('img') && t.getAttribute('aria-pressed') === 'false'), cid));
    check('… and greys the card in the result', (await pg.locator('.deck .dc.off').count()) > 0);
    await pg.locator('.cpop .sw').click(); await pg.keyboard.press('Escape');
    check('Escape closes the panel', await pg.evaluate(() => document.querySelector('.cpop').hidden));
    // an outfit skill under a member: its icon, a plain label (no panel)
    const sp = pg.locator('.loop').first().locator('.mem .spark').first();
    check('outfit skill shows its skill icon, not clickable', (await sp.locator('img.sic').count()) === 1 && await sp.evaluate(e => e.tagName === 'SPAN'));
    const mb = pg.locator('.loop').first().locator('.mem .memb').nth(1); await mb.click(); await pg.waitForTimeout(100);
    check('member panel has one Owned switch', (await pg.locator('.cpop input[role=switch]').count()) === 1);
    const uh = await pg.locator('.cpop .gtl').getAttribute('href');
    check('member panel: GameTora character link', /^https:\/\/gametora\.com\/umamusume\/characters\/\d{6}-[a-z0-9-]+$/.test(uh || ''), uh);
    await pg.locator('.cpop .sw').click(); await pg.waitForTimeout(100);
    check('member Owned switch updates the roster and greys her', (await pg.locator('#roster .ut[data-s="out"]').count()) === 1 && (await pg.locator('.mem.off').count()) > 0);
    await pg.locator('.cpop .sw').click(); await pg.keyboard.press('Escape'); await settle(pg);
    // must include, then Owned off in her panel: she is not owned and no longer must include (no contradiction left)
    const m0 = pg.locator('.loop').first().locator('.mem').first(), uid = await m0.getAttribute('data-uma'), anyOutfit = !(await m0.getAttribute('data-outfit'));
    const tile = pg.locator(`#roster .ut[data-uma="${uid}"] .up`).first(); await tile.click(); await tile.click(); await settle(pg);
    const marked = await pg.locator(`#roster .ut[data-uma="${uid}"][data-s="req"]`).count();
    await pg.locator('.loop').first().locator(`.mem[data-uma="${uid}"] .memb`).click(); await pg.locator('.cpop .sw').click(); await pg.keyboard.press('Escape'); await settle(pg);
    const after = await pg.evaluate((id) => [...document.querySelectorAll(`#roster .ut[data-uma="${id}"]`)].map(t => t.dataset.s), uid);
    check('must include, then Owned off in the member panel: not owned, no longer must include, and the search runs', anyOutfit && marked === 1 && after.every(x => x === 'out') && /Searched/.test(await note(pg)), after.join(',') + ' · ' + await note(pg));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async targets(b) {
    // the target list by keyboard: removing keeps the focus in the list, the add box takes Enter and the arrows
    const pg = await open(b);
    const skl = await json(pg, 'data-skills'), gold = (g) => skl.find(x => x[0] === g)[2], white = (g) => skl.find(x => x[0] === g)[1];
    // guarantees follow the scenario (no per-target control); the add box sits above the list, two targets per row
    const lay = await pg.evaluate(() => { const t = [...document.querySelectorAll('#targets .tgt')].map(e => e.getBoundingClientRect());
      return { selects: document.querySelectorAll('#targets select').length, head: !!document.getElementById('t-head'), rows: new Set(t.map(r => Math.round(r.top))).size, n: t.length,
        addFirst: document.getElementById('t-add').getBoundingClientRect().bottom <= Math.min(...t.map(r => r.top)) }; });
    check('targets: no guaranteed selector or column header, add box first, two per row', lay.selects === 0 && !lay.head && lay.addFirst && lay.rows === Math.ceil(lay.n / 2), JSON.stringify(lay));
    check('a gold the scenario guarantees shows as the gold itself, in gold', (await pg.locator('#g-20049 .name').innerText()).trim() === gold(20049) && await pg.locator('#g-20049').evaluate(e => e.classList.contains('gold')), await pg.locator('#g-20049').innerText());
    await pg.locator('#targets .x').nth(2).focus(); await pg.keyboard.press('Enter'); await pg.waitForTimeout(100);
    check('removing a target by keyboard moves the focus to the next remove button', (await pg.locator('#targets .tgt').count()) === 7
      && await pg.evaluate(() => document.activeElement.classList.contains('x') && [...document.querySelectorAll('#targets .x')].indexOf(document.activeElement) === 2));
    for (let i = 0; i < 7; i++) await pg.keyboard.press('Enter');
    check('… and to the add box after the last one', (await pg.locator('#targets .tgt').count()) === 0 && await pg.evaluate(() => document.activeElement.id === 't-add'));
    await pg.fill('#t-add', 'corner'); await pg.waitForTimeout(100);
    const first = await pg.locator('#t-sugg button').first().innerText();
    await pg.keyboard.press('ArrowDown'); await pg.keyboard.press('ArrowDown'); await pg.keyboard.press('ArrowUp');
    const at = await pg.evaluate(() => [...document.querySelectorAll('#t-sugg button')].indexOf(document.activeElement));
    await pg.keyboard.press('ArrowUp'); const back = await pg.evaluate(() => document.activeElement.id);
    check('arrows walk the suggestions, Up from the first returns to the box', at === 0 && back === 't-add', at + ' ' + back);
    await pg.keyboard.press('ArrowDown'); await pg.keyboard.press('Escape');
    check('Escape on a suggestion closes the list and returns to the box', await pg.locator('#t-sugg').isHidden() && await pg.evaluate(() => document.activeElement.id === 't-add'));
    await pg.fill('#t-add', 'corner'); await pg.keyboard.press('Enter'); await pg.waitForTimeout(100);
    check('Enter adds the first match and keeps the focus in the box', (await pg.locator('#targets .tgt').count()) === 1 && (await pg.locator('#targets .tgt').innerText()).includes(first.split(' → ')[0]) && await pg.evaluate(() => document.activeElement.id === 't-add'), first);
    await pg.fill('#t-add', first.split(' → ')[0].slice(0, 8)); await pg.waitForTimeout(100);
    check('a skill already listed: "… is already a target"', (await pg.locator('#t-sugg .snote').allInnerTexts()).some(t => t === first.split(' → ')[0] + ' is already a target'));
    await pg.fill('#t-add', 'zzqx'); await pg.waitForTimeout(100);
    check('no match: "No skill matches"', /^No skill matches/.test(await pg.locator('#t-sugg').innerText()));
    // a gold matched by its own name shows with its white
    const g = skl.find(x => x[2] && !x[1].toLowerCase().includes(x[2].toLowerCase().slice(0, 6)) && !skl.some(y => y !== x && (y[1] + (y[2] || '')).toLowerCase().includes(x[2].toLowerCase())));
    await pg.fill('#t-add', g[2]); await pg.waitForTimeout(100);
    check('a match on the gold\'s name shows "white → gold"', (await pg.locator('#t-sugg button').first().innerText()) === g[1] + ' → ' + g[2], g[1] + ' → ' + g[2]);
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async keyboard(b) {
    const pg = await open(b);
    const act = () => pg.evaluate(() => { const a = document.activeElement; return a.id || a.className || a.tagName; });
    await pg.keyboard.press('Tab');
    const skip = await pg.evaluate(() => { const a = document.activeElement, r = a.getBoundingClientRect(); return { cls: a.className, top: r.top, text: a.textContent }; });
    await pg.keyboard.press('Enter');
    check('the first Tab stop is a visible "Skip to results" link that moves the focus to the results', skip.cls === 'skip' && skip.top >= 0 && skip.text === 'Skip to results' && (await act()) === 'rcol', JSON.stringify(skip));
    // the deck card panel: focus moves in on open; Tab past its end returns to the card
    await pg.locator('.deck .dc').first().focus(); await pg.keyboard.press('Enter');
    const inPop = await pg.evaluate(() => document.querySelector('.cpop').contains(document.activeElement) && document.activeElement.getAttribute('role') === 'switch');
    await pg.keyboard.press('Tab'); await pg.keyboard.press('Tab');
    check('deck card panel: opening focuses its Owned switch; tabbing out closes it and returns to the card', inPop && await pg.evaluate(() => document.querySelector('.cpop').hidden && document.activeElement.classList.contains('dc')));
    await pg.locator('.mem .memb').first().focus(); await pg.keyboard.press('Enter');
    const memIn = await pg.evaluate(() => document.activeElement.getAttribute('role') === 'switch'); await pg.keyboard.press('Shift+Tab');
    check('member panel: the same (Shift+Tab out returns to her name)', memIn && await pg.evaluate(() => document.querySelector('.cpop').hidden && document.activeElement.classList.contains('memb')));
    // sort menu: the checked item takes the focus, arrows move, Escape returns
    await pg.focus('#rs'); await pg.keyboard.press('Enter');
    const checked = await pg.evaluate(() => document.activeElement.getAttribute('aria-checked'));
    await pg.keyboard.press('ArrowDown'); const moved = await pg.evaluate(() => document.activeElement.dataset.k);
    await pg.keyboard.press('Escape');
    check('sort menu: opens on the checked item, arrows move, Escape closes back to the button', checked === 'true' && moved === 'name' && await pg.locator('#rs-menu').isHidden() && (await act()) === 'rs', checked + ' ' + moved);
    // schedule tabs: arrows move and select
    await pg.locator('.loop .lacts button', { hasText: 'Race schedule' }).first().click(); await pg.waitForTimeout(200);
    await pg.locator('#sched-dlg .stab').first().focus(); await pg.keyboard.press('ArrowRight');
    const tabs = await pg.evaluate(() => [...document.querySelectorAll('#sched-dlg .stab')].map(t => [t.getAttribute('aria-selected'), t.tabIndex, t.getAttribute('aria-controls')].join(':')));
    check('schedule tabs: ArrowRight selects and focuses the next tab, one tab in the Tab order, each controls the panel', tabs[1] === 'true:0:sched-view' && tabs.filter(t => t.startsWith('true')).length === 1
      && await pg.evaluate(() => document.activeElement === document.querySelectorAll('#sched-dlg .stab')[1]), tabs.join(' '));
    await pg.keyboard.press('Escape');
    // affinity: each career is a button
    await pg.locator('.loop .lacts button', { hasText: 'Affinity' }).first().click(); await pg.waitForTimeout(200);
    const rb = pg.locator('#why-dlg .rowbtn[aria-pressed="false"]').first(); await rb.focus(); await pg.keyboard.press('Enter');
    check('affinity: the careers are buttons; Enter shows one', (await pg.locator('#why-dlg .rowbtn').count()) === 12 && await pg.evaluate(() => document.activeElement.getAttribute('aria-pressed') === 'true' && document.activeElement.closest('tr').classList.contains('sel'))
      && /choose one/i.test(await pg.locator('#why-dlg').innerText()));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async three(b) {
    // the result's wording, and three results told apart: #1/#2/#3, "Same loop as #1", the deck cards #1 lacks marked
    const pg = await open(b);
    const head = await pg.evaluate(() => { const l = document.querySelector('#results .loop'); return { odds: l.querySelector('.odds').innerText, sub: l.querySelector('.sub').innerText, rank: !!l.querySelector('.rank'),
      own: [...l.querySelectorAll('.mem .own')].map(e => e.innerText), font: getComputedStyle(l.querySelector('.odds')).fontFamily }; });
    check('headline: "1 in 139 runs to land all 8", then "at least 7 of 8" and the 90% line, in the UI font; no rank with one result',
      /^1 in 139 runs to land all 8$/.test(head.odds) && /^at least 7 of 8: 1 in 15\.7 · 90% sure within 319 runs$/.test(head.sub) && !head.rank && /Zen Kaku/.test(head.font), JSON.stringify(head));
    check('members: "as trainee: 1 in …"', head.own.length === 4 && head.own.every(x => /^as trainee: 1 in [\d,.]+$/.test(x)), head.own.join(' | '));
    await pg.selectOption('#c-top', '3'); await pg.click('#c-peruma'); await settle(pg);
    const r = await pg.evaluate(() => { const L = [...document.querySelectorAll('#results .loop')], key = (l) => [...l.querySelectorAll('.mem')].map(m => m.dataset.uma + ':' + m.dataset.outfit).join(' ');
      const deck = (l) => [...l.querySelector('.deck .dcards').querySelectorAll('.dc')].map(d => d.dataset.cid);
      return L.map((l, i) => ({ rank: (l.querySelector('.rank') || {}).textContent, same: !!l.querySelector('.same'), mem: l.querySelectorAll('.mem').length,
        sameAs1: i > 0 && !l.querySelector('.same') && key(l) === key(L[0]),
        noNew: !l.querySelector('.dc.new, .dnew') })); });
    check('three results: ranked #1, #2, #3', r.map(x => x.rank).join() === '#1,#2,#3', JSON.stringify(r));
    check('a result with #1\'s loop says "Same loop as #1" instead of repeating the umas', r.some(x => x.same) && r.every(x => x.same ? x.mem === 0 : x.mem === 4) && !r.some(x => x.sameAs1), JSON.stringify(r));
    check('#2 and #3 mark no cards as new', r.every(x => x.noNew), JSON.stringify(r));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async states(b) {
    // what the status line says when the settings can't be searched, and the results on screen meanwhile (kept, dimmed)
    const pg = await open(b);
    const head = () => pg.evaluate(() => ({ kind: document.getElementById('rhead').dataset.kind, note: document.getElementById('run-note').textContent, act: document.getElementById('r-act').innerText,
      loops: document.querySelectorAll('#results .loop').length, stale: document.getElementById('results').classList.contains('stale') }));
    while (await pg.locator('#targets .x').count()) await pg.locator('#targets .x').first().click();
    await pg.waitForTimeout(700); let h = await head();
    check('no targets: a neutral status with a button that loads the example targets; the old result stays, dimmed', h.kind === 'empty' && /Add a target skill/.test(h.note) && /8\/8 example/.test(h.act) && h.loops > 0 && h.stale, JSON.stringify(h));
    await pg.click('#r-act button'); await settle(pg); h = await head();
    check('… the button loads them and the search runs', (await pg.locator('#targets .tgt').count()) === 8 && h.kind === 'done' && /Searched/.test(h.note) && !h.stale, JSON.stringify(h));
    await pg.click('#r-none'); await pg.waitForTimeout(700); h = await head();
    check('no umas owned: a neutral status with "Mark all umas owned"', h.kind === 'empty' && /at least 4 umas/.test(h.note) && /Mark all umas owned/.test(h.act) && h.loops > 0, JSON.stringify(h));
    await pg.click('#r-act button'); await settle(pg);
    // contradictory settings (an import can carry them): the status says what is wrong, the result stays
    const u = (await json(pg, 'data-umas')).find(x => x.outfits.length > 1);
    await pg.click('#ex-open'); const ex = JSON.parse(await pg.inputValue('#ex-text'));
    ex.settings.roster = { [u.id]: 'out' }; ex.settings.outfits = { [u.outfits[0].id]: 'req' };
    await pg.fill('#ex-text', JSON.stringify(ex)); await pg.click('#ex-load'); await pg.click('#ex-close'); await pg.waitForTimeout(700); h = await head();
    check('must include but not owned: an error naming her, in the status line; results kept', h.kind === 'error' && h.note.includes(u.name + ' is marked Must include but not owned') && h.loops > 0 && h.stale, JSON.stringify(h));
    if (b.browserType().name() === 'chromium') { const cf = (await pg.evaluate(AUDIT)).filter(x => /^p\.runnote/.test(x)); check('contrast: the error status line meets WCAG AAA', !cf.length, cf.join(' | ')); }
    await pg.click('#r-all'); await settle(pg);
    // Stop: a slow search (three results, one shared deck) is stopped; the results stay and the worker is killed
    const r0 = await pg.evaluate(() => window.__runs.length);
    await pg.selectOption('#c-top', '3'); await pg.click('#c-peruma');
    await pg.waitForFunction((n) => window.__runs.length > n, r0);
    check('while searching: the results are marked busy and a Stop button shows', await pg.evaluate(() => document.getElementById('results').getAttribute('aria-busy') === 'true') && (await pg.locator('#r-act button').innerText()) === 'Stop');
    const k0 = await pg.evaluate(() => window.__killed); await pg.click('#r-act button'); await pg.waitForTimeout(300); h = await head();
    check('Stop kills the search and keeps the results', (await pg.evaluate(() => window.__killed)) > k0 && h.kind === 'stopped' && h.loops > 0 && !(await busy(pg)), JSON.stringify(h));
    await pg.click('#r-act button'); await settle(pg);
    check('… and "Search again" runs it', /Searched/.test(await note(pg)) && (await pg.locator('#results .loop').count()) === 3);
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async borrowed(b) {
    const pg = await open(b);
    for (const nm of ['Light Hello SSR', 'Sasami Anshinzawa SSR']) await pg.locator('#cards .ct', { hasText: nm }).first().click();
    await settle(pg);
    const rows = await pg.evaluate(() => [...document.querySelectorAll('.deck .dcards')].map(r => [...r.querySelectorAll('.dc')]).filter(r => r.some(x => x.classList.contains('bor'))).map(r => r.findIndex(x => x.classList.contains('bor')) === r.length - 1));
    check('a deck borrows a card when the pals are not owned', rows.length > 0);
    check('the borrowed card is last in its row', rows.every(Boolean));
    check('the borrowed card is named as borrowed (no caption under its art)', await pg.evaluate(() => [...document.querySelectorAll('.dc.bor')].every(d => /borrowed/.test(d.getAttribute('aria-label')) && ![...d.querySelectorAll('i')].some(i => i.textContent === 'borrowed'))));
    check('the borrowed card has the friend frame', await pg.evaluate(() => { const d = document.querySelector('.dc.bor'); return !!d && getComputedStyle(d, '::after').borderImageSource.startsWith('url'); }));
    check('deck headers carry no counts', !(await pg.evaluate(() => [...document.querySelectorAll('.deck b')].some(x => /· \d+ pal|borrow 1/.test(x.textContent)))));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async popups(b) {
    const pg = await open(b);
    await pg.locator('.loop .lacts button', { hasText: 'Race schedule' }).first().click(); await pg.waitForTimeout(250);
    check('schedule: a tab per member plus All four', (await pg.locator('#sched-dlg .stab').count()) === 5);
    const grids = await pg.evaluate(() => [...document.querySelectorAll('.ag-grid')].map(g => { const r = g.getBoundingClientRect(); return Math.round(r.width) + 'x' + Math.round(r.height); }));
    check('agenda: three identical year grids', grids.length === 3 && new Set(grids).size === 1, grids.join(' '));
    check('agenda: fits the window without scrolling', await pg.evaluate(() => { const d = document.querySelector('#sched-dlg .dlg'); return d.scrollHeight <= d.clientHeight + 1; }));
    check('agenda: every race has its banner', (await pg.locator('#sched-dlg .ag-race').count()) === (await pg.locator('#sched-dlg .ag-race img.banner').count()));
    check('agenda: a race\'s win % sits in its turn bar, not over the banner', (await pg.locator('#sched-dlg .ag-p').count()) === 0
      && await pg.evaluate(() => [...document.querySelectorAll('#sched-dlg .ag-slot.on .ag-when')].every(w => /^(Early|Late) [A-Z][a-z]{2}\d+%$/.test(w.textContent))));
    await pg.locator('#sched-dlg .stab').nth(4).click(); check('All four shows the comparison table', (await pg.locator('#sched-dlg table.sched').count()) > 0);
    check('All four: turns written as on the member tabs ("Early Apr"), no shared/optional key', await pg.evaluate(() => [...document.querySelectorAll('#sched-dlg td.when')].every(t => /^(Early|Late) [A-Z][a-z]{2}$/.test(t.textContent)))
      && await pg.locator('#sched-dlg .agkey').isHidden());
    await pg.keyboard.press('Escape');
    await pg.locator('.loop .lacts button', { hasText: 'Affinity' }).first().click(); await pg.waitForTimeout(250);
    const rows = await pg.evaluate(() => [...document.querySelectorAll('#why-dlg table.tbl')].map(t => t.tBodies[0].rows.length));
    check('affinity: 6 pairs and 12 careers', rows.join(',') === '6,12', rows.join(','));
    check('affinity: no odds arithmetic', !(await pg.evaluate(() => /Two rolls|All targets land/.test(document.getElementById('why-dlg').innerText))));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async phone(b) {
    const pg = await open(b, { w: 390, h: 844, mobile: true });
    check('phone: no sideways scroll', !(await pg.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth)));
    const small = await pg.evaluate(() => [...document.querySelectorAll('.rail button, .rail select, .rail input[type=search], #rhead button, #rhead select, .loop .lacts button, .mem .memb')]
      .filter(e => e.offsetParent && !e.closest('.roster, .cards')).map(e => [e.id || e.className || e.tagName, Math.round(e.getBoundingClientRect().height)]).filter(([, h]) => h < 44));
    check('phone: buttons, selects and search boxes are at least 44 px high', !small.length, JSON.stringify(small.slice(0, 6)));
    // results first: above the settings, and first in the document (reading and Tab order), the result in the first screen
    const order = await pg.evaluate(() => { const c = document.getElementById('rcol'), r = document.querySelector('.rail'), o = document.querySelector('#results .loop .odds').getBoundingClientRect();
      return { above: c.getBoundingClientRect().top < r.getBoundingClientRect().top, dom: !!(c.compareDocumentPosition(r) & Node.DOCUMENT_POSITION_FOLLOWING), odds: Math.round(o.bottom), h: innerHeight }; });
    check('phone: the results come before the settings (on screen and in the Tab order), the odds in the first screen', order.above && order.dom && order.odds < order.h, JSON.stringify(order));
    if (b.browserType().name() === 'chromium') { const cls = await pg.evaluate(() => new Promise(r => { let v = 0; new PerformanceObserver(l => { for (const e of l.getEntries()) if (!e.hadRecentInput) v += e.value; }).observe({ type: 'layout-shift', buffered: true }); setTimeout(() => r(v), 200); }));
      check('phone: the page barely shifts while loading (CLS < 0.05)', cls < 0.05, cls.toFixed(4)); }
    // the floating status button: shown only while the results are out of view, on screen wherever the page is scrolled
    const H = await pg.evaluate(() => document.documentElement.scrollHeight); const seen = [];
    for (const f of [0, 0.5, 1]) { await pg.evaluate((y) => scrollTo(0, y), f * H); await pg.waitForTimeout(150);
      seen.push(await pg.evaluate(() => { const p = document.getElementById('run-pill'), r = p.getBoundingClientRect(), c = document.getElementById('rcol').getBoundingClientRect();
        const resIn = c.bottom > 0 && c.top < innerHeight, shown = getComputedStyle(p).display !== 'none'; return shown === !resIn && (!shown || (r.top >= 0 && r.bottom <= innerHeight)); })); }
    check('phone: the status button shows (on screen) exactly when the results are out of view', seen.every(Boolean), seen.join(','));
    check('phone: it shows the headline odds', /^1 in 139 · see result/.test(await pg.locator('#run-pill').innerText()), await pg.locator('#run-pill').innerText());
    // edit a setting far down, then the button takes you back to the result
    const ct = pg.locator('#cards .ct').nth(2); await ct.scrollIntoViewIfNeeded(); await ct.click(); await settle(pg); await ct.click(); await settle(pg);
    await pg.click('#run-pill'); await pg.waitForTimeout(900);
    check('phone: after editing a card, the status button scrolls back to the results', await pg.evaluate(() => { const r = document.getElementById('rcol').getBoundingClientRect(); return Math.abs(r.top) < 40 && document.activeElement.id === 'rcol'; }));
    // the roster and card boxes: shorter than the screen, their bottom edge fading while there is more below
    const boxes = await pg.evaluate(() => ['roster', 'cards'].map(id => { const e = document.getElementById(id); return { h: e.clientHeight / innerHeight, more: e.classList.contains('more') }; }));
    check('phone: the roster and card boxes are at most 55% of the screen high and show that they scroll', boxes.every(x => x.h <= 0.56 && x.more), JSON.stringify(boxes));
    await pg.evaluate(() => scrollTo(0, 0));
    await pg.click('#x-scen');
    check('phone: the open scenario picker fits the screen', await pg.evaluate(() => { const r = document.getElementById('scen-menu').getBoundingClientRect();
      return r.width > 0 && r.left >= 0 && r.right <= innerWidth && document.documentElement.scrollWidth <= document.documentElement.clientWidth; }));
    const ph = await pg.evaluate(() => [...document.querySelectorAll('#scen-menu .stile')].map(t => { const r = t.getBoundingClientRect(), s = t.querySelector('.ssub').getBoundingClientRect();
      return Math.round(r.width) + 'x' + Math.round(r.height) + ' line ' + Math.round(s.height); }));
    check('phone: the scenario tiles are the same size, each logo + one line', new Set(ph.map(x => x.split(' ')[0])).size === 1 && ph.every(x => +x.split(' ').at(-1) <= 22), ph.join(' | '));
    await pg.keyboard.press('Escape');
    await pg.locator('.loop .lacts button', { hasText: 'Race schedule' }).first().click(); await pg.waitForTimeout(250);
    check('phone: agenda stacks without the blank Junior half-year', (await pg.locator('#sched-dlg .ag-slot.none:visible').count()) === 0);
    await pg.evaluate(() => { const d = document.querySelector('#sched-dlg .dlg'); d.scrollTo(0, d.scrollHeight); }); await pg.waitForTimeout(100);
    const closeVis = () => pg.evaluate(() => { const b = [...document.querySelectorAll('dialog[open] .dhead button')].find(x => x.textContent === 'Close'), r = b.getBoundingClientRect(); return r.top >= 0 && r.bottom <= innerHeight; });
    check('phone: the schedule\'s Close stays on screen at the bottom of the dialog', await closeVis());
    await pg.locator('#sched-dlg .stab').nth(4).click(); await pg.waitForTimeout(100);
    check('phone: the All four table fits the 390 px screen', await pg.evaluate(() => [...document.querySelectorAll('#sched-dlg .tablewrap')].every(w => w.scrollWidth <= w.clientWidth)));
    await pg.keyboard.press('Escape');
    await pg.locator('.loop .lacts button', { hasText: 'Affinity' }).first().click(); await pg.waitForTimeout(250);
    await pg.evaluate(() => { const d = document.querySelector('#why-dlg .dlg'); d.scrollTo(0, d.scrollHeight); }); await pg.waitForTimeout(100);
    check('phone: the affinity\'s Close stays on screen too', await closeVis());
    await pg.keyboard.press('Escape');
    await pg.click('#ex-open'); check('the export dialog\'s Close sits in the same header row as the others', await pg.evaluate(() => document.getElementById('ex-close').closest('.dhead') !== null)); await pg.keyboard.press('Escape');
    await pg.setViewportSize({ width: 1280, height: 800 }); await pg.waitForTimeout(150);
    check('widened to desktop, the results go back beside (after) the settings', await pg.evaluate(() => !!(document.querySelector('.rail').compareDocumentPosition(document.getElementById('rcol')) & Node.DOCUMENT_POSITION_FOLLOWING)));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async nonecards(b) {
    // "None owned" on support cards: no owned card reaches the solver (an empty owned list used to mean "all owned")
    const ctx = await b.newContext(); const pg = await ctx.newPage();
    await ctx.addInitScript(() => { window.__jobs = []; const W = window.Worker; window.Worker = function (...a) { const w = new W(...a); const pm = w.postMessage.bind(w);
      w.postMessage = (m, ...r) => { if (m && m.type === 'run') window.__jobs.push(m.job); return pm(m, ...r); }; return w; }; });
    const done = () => pg.waitForFunction(() => /Searched/.test(document.getElementById('run-note').textContent), null, { timeout: 60000 });
    await pg.goto(URL); await done(); const all = await odds(pg);
    await pg.click('#c-none'); await pg.click('#c-borrow'); await pg.waitForTimeout(700); await done();
    check('None owned: the count says "0 of N listed cards owned"', /^0 of \d+ listed cards owned$/.test(await pg.locator('#c-count').innerText()), await pg.locator('#c-count').innerText());
    const job = await pg.evaluate(() => window.__jobs.at(-1));
    check('None owned + no borrowing: the job allows no card and the odds drop', job.owned_cards.length === 1 && job.owned_cards[0] === 0 && !job.borrow && (await odds(pg)) !== all, (await odds(pg)) + ' vs ' + all);
    await ctx.close();
  },
  async reload(b) {
    // a reload with non-default settings shows their last result from the first frame (not the default example), and
    // the fresh identical result doesn't redraw it
    const ctx = await b.newContext(); const pg = await ctx.newPage(); const errs = []; pg.on('pageerror', e => errs.push(e.message));
    await pg.goto(URL); await pg.waitForFunction(() => /Searched/.test(document.getElementById('run-note').textContent), null, { timeout: 60000 });
    await pg.click('#x-scen'); await pg.click('#scen-menu .stile[data-v="gc"]'); await pg.waitForTimeout(600); await pg.waitForFunction(() => /Searched/.test(document.getElementById('run-note').textContent), null, { timeout: 60000 });
    const mine = await odds(pg);
    await ctx.addInitScript(() => { window.__seen = []; const snap = () => { const o = document.querySelector('#results .loop .odds'); window.__seen.push(o ? o.textContent : '');
      if (o && !o.__mark) o.__mark = 1; if (performance.now() < 6000) requestAnimationFrame(snap); }; requestAnimationFrame(snap); });
    await pg.reload(); await pg.waitForFunction(() => /Searched/.test(document.getElementById('run-note').textContent), null, { timeout: 60000 });
    const seen = [...new Set(await pg.evaluate(() => window.__seen))];
    check('reload: the saved settings\' result from the first frame, never the default example', seen.length === 1 && seen[0] === mine && mine !== '' && !/1 in 139runs/.test(seen[0]), seen.join(' | '));
    check('reload: an identical fresh result is not redrawn', await pg.evaluate(() => document.querySelector('#results .loop .odds').__mark === 1));
    check('no page errors', !errs.length, errs.join(' | '));
    await ctx.close();
  },
  async delivery(b) {
    // self-hosted fonts (no third-party requests), a real favicon, the solver's files fingerprinted, the first roster
    // portraits eager, and accessible names that start with the visible text (voice control)
    const ctx = await b.newContext(); const pg = await ctx.newPage(); const hosts = new Set(); const urls = [];
    pg.on('request', r => { const u = new globalThis.URL(r.url()); if (u.protocol.startsWith('http')) { hosts.add(u.host); urls.push(u.pathname); } });
    await pg.goto(URL); await pg.waitForFunction(() => /Searched/.test(document.getElementById('run-note').textContent), null, { timeout: 60000 });
    check('no third-party requests (fonts self-hosted)', hosts.size === 1, [...hosts].join(' '));
    check('the solver data and binary come from fingerprinted files', urls.some(u => /^\/assets\/engine-global\.[0-9a-f]{10}\.json$/.test(u)) && urls.some(u => /^\/assets\/loopopt\.[0-9a-f]{10}\.wasm$/.test(u)), urls.filter(u => u.startsWith('/assets/') && !u.includes('/img.')).join(' '));
    check('page fonts are the self-hosted subsets', await pg.evaluate(async () => { await document.fonts.ready; return [...document.fonts].some(f => f.family.includes('Zen Kaku') && f.status === 'loaded'); }));
    const ico = await pg.request.get(new globalThis.URL('favicon.ico', URL).href);
    check('favicon.ico is an icon, not the page', ico.ok() && /image\//.test(ico.headers()['content-type'] || ''), ico.headers()['content-type']);
    check('the first roster portraits load eagerly', await pg.evaluate(() => { const im = [...document.querySelectorAll('#roster .av img')]; return im.length > 8 && im.slice(0, 8).every(i => i.loading === 'eager') && im.slice(8).every(i => i.loading === 'lazy'); }));
    check('sort buttons are named by their visible text first', await pg.evaluate(() => ['rs', 'cs'].every(id => { const e = document.getElementById(id); return e.getAttribute('aria-label').startsWith(e.textContent.trim()); })));
    check('card tiles take their name from their visible text', await pg.evaluate(() => [...document.querySelectorAll('#cards .ct')].every(e => !e.hasAttribute('aria-label') && e.hasAttribute('aria-pressed'))));
    await ctx.close();
  },
  async eraswitch(b) {
    // the era buttons: the Grand Masters-era data opens on Grand Masters (rank SS, no Unity golds) and searches with it;
    // back on Global the scenario is Unity Cup again (golds guaranteed, rank below SS)
    const pg = await open(b);
    await pg.click('#era-gm'); await pg.waitForLoadState('load'); await settle(pg);
    check('switching to the GM era selects Grand Masters', (await pg.getAttribute('#x-scen', 'data-value')) === 'grandmasters' && (await pg.getAttribute('#rank', 'data-value')) === '0.20,0.70,0.10');
    check('the GM-era search runs with Grand Masters', /Searched/.test(await note(pg)) && await pg.evaluate(() => window.__runs.length > 0 && window.__runs.at(-1).scenario === 'grandmasters'));
    check('Grand Masters gives no guaranteed golds', (await pg.getAttribute('#g-20049', 'data-g')) === '0' && (await pg.getAttribute('#g-20046', 'data-g')) === '0');
    await pg.click('#era-global'); await pg.waitForLoadState('load'); await settle(pg);
    check('back on Global: Unity Cup with its golds, rank below SS', (await pg.getAttribute('#x-scen', 'data-value')) === 'unity' && (await pg.getAttribute('#rank', 'data-value')) === '0.50,0.45,0.05'
      && (await pg.getAttribute('#g-20049', 'data-g')) === '2' && await pg.evaluate(() => window.__runs.at(-1).scenario === 'unity'));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async gm(b) {
    const pg = await open(b, { init: () => { try { if (!sessionStorage.getItem('gm')) { sessionStorage.setItem('gm', '1'); localStorage.setItem('llp:era', 'gm'); } } catch {} } });
    await pg.route('**/assets/data-gm.*', async (r) => { await new Promise(x => setTimeout(x, 1500)); await r.continue(); });   // a slow fetch of the GM era's data
    await pg.reload(); await pg.waitForTimeout(500);
    check('GM era: while its data loads, the status line says so', /Loading the JP Grand Masters-era data/.test(await note(pg)), await note(pg));
    await settle(pg); await pg.unroute('**/assets/data-gm.*');
    const umas = (await pg.evaluate(() => fetch(JSON.parse(document.getElementById('data-assets').textContent).dataGm).then(r => r.json()))).umas; const nOut = umas.reduce((a, u) => a + Math.max(1, u.outfits.length), 0);
    check('GM era: roster from the GM data', (await pg.locator('#roster .ut').count()) === nOut, nOut + ' outfits');
    check('GM era: the search runs', /Searched/.test(await note(pg)) && (await pg.locator('#results .loop').count()) > 0, await odds(pg));
    // the GM era offers Grand Masters as a fifth tile (same size as the others), and its search carries it
    await pg.click('#x-scen');
    const gt = await pg.evaluate(() => [...document.querySelectorAll('#scen-menu .stile')].map(t => { const r = t.getBoundingClientRect(); return { v: t.dataset.v, sub: t.querySelector('.ssub').innerText.replace(/\s+/g, ' ').trim(), size: Math.round(r.width) + 'x' + Math.round(r.height), x: Math.round(r.left + r.width / 2) }; }));
    const mid = await pg.evaluate(() => { const r = document.getElementById('scen-menu').getBoundingClientRect(); return Math.round(r.left + r.width / 2); });
    check('GM era: five equal tiles, Grand Masters last and centred', gt.length === 5 && gt[4].v === 'grandmasters' && gt[4].sub === 'Hint rate ×1.44 (estimate)' && new Set(gt.map(t => t.size)).size === 1 && Math.abs(gt[4].x - mid) <= 1, JSON.stringify(gt));
    if (process.env.LLP_SHOTS) await pg.locator('#scen-menu').screenshot({ path: `${process.env.LLP_SHOTS}/${b.browserType().name()}-scen-picker-gm.png` });
    await pg.click('#scen-menu .stile[data-v="grandmasters"]'); await settle(pg);
    check('GM era: picking Grand Masters searches with it, at rank SS', await pg.evaluate(() => window.__runs.at(-1).scenario === 'grandmasters' && window.__runs.at(-1).star_dist.join() === '0.2,0.7,0.1')
      && (await pg.getAttribute('#x-scen', 'aria-label')) === 'Scenario: Grand Masters');
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async settings(b) {
    // a browser with the old per-key settings: everything carries over into llp:settings, old keys are removed
    const pg = await open(b, { init: () => { try { if (sessionStorage.getItem('seeded')) return; sessionStorage.setItem('seeded', '1'); localStorage.clear();
      const set = (k, v) => localStorage.setItem(k, JSON.stringify(v));
      set('llp:roster', { 1001: 'out' }); set('llp:gm:roster', { 1002: 'out' }); set('llp:cardsOff', { 30052: false });
      set('llp:targets', [{ group: 20166, guaranteed: 2, deck_gold: 1 }, { group: 20159, guaranteed: 0, deck_white: 0.5 }]);
      set('llp:rank', '0.45,0.50,0.05'); set('llp:rsort', 'rel'); set('llp:rdir', 'asc'); localStorage.setItem('llp:era', 'global'); } catch {} } });
    const st = await pg.evaluate(() => ({ keys: Object.keys(localStorage).filter(k => k.startsWith('llp:') && !k.startsWith('llp:last:')), s: JSON.parse(localStorage.getItem('llp:settings')) }));
    check('migration: one llp:settings object, old keys removed', st.keys.length === 1 && st.keys[0] === 'llp:settings' && st.s.v === 2, st.keys.join(','));
    check('migration: per-era roster kept apart', st.s.global.roster['1001'] === 'out' && st.s.gm.roster['1002'] === 'out');
    check('migration: the uma marked not owned shows not owned', await pg.evaluate(() => [...document.querySelectorAll('#roster .ut[data-uma="1001"]')].every(t => t.dataset.s === 'out')));
    check('migration: card ownership kept', await pg.evaluate(() => document.querySelectorAll('#cards .ct[aria-pressed="false"]').length === 1));
    check('migration: old preset gold fixed, hand-set odds dropped', JSON.stringify(st.s.common.targets) === JSON.stringify([{ group: 20166, guaranteed: 0 }, { group: 20159, guaranteed: 0 }]));
    check('migration: old star mix remapped', (await pg.getAttribute('#rank', 'data-value')) === '0.50,0.45,0.05');
    check('migration: roster sort reset to newest first', (await pg.locator('#rs').innerText()) === 'Release ↓');
    // changes persist across a reload
    await pg.locator('#roster .ut .up').nth(3).click(); await settle(pg); await pg.reload(); await settle(pg);
    check('a change survives a reload', (await pg.locator('#roster .ut[data-s="out"]').count()) >= 1 + (await pg.locator('#roster .ut[data-uma="1001"]').count()));
    // export, then import the same text
    await pg.click('#ex-open'); const text = await pg.inputValue('#ex-text');
    await pg.fill('#ex-text', text); await pg.click('#ex-load');
    check('export then import: settings load', (await pg.locator('#ex-msg').innerText()) === 'Settings loaded.' && JSON.parse(text).settings.era === 'global');
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
    // settings saved before R cards became opt-in: a saved "on" (almost always the old default) is reset once
    const p2 = await open(b, { init: () => { try { if (sessionStorage.getItem('seeded')) return; sessionStorage.setItem('seeded', '1'); localStorage.clear();
      localStorage.setItem('llp:settings', JSON.stringify({ v: 1, era: 'global', common: { rcards: true, rank: '0.20,0.70,0.10' }, global: {}, gm: {} })); } catch {} } });
    const p3 = await open(b, { init: () => { try { if (sessionStorage.getItem('seeded')) return; sessionStorage.setItem('seeded', '1'); localStorage.clear();
      localStorage.setItem('llp:settings', JSON.stringify({ v: 2, era: 'global', common: { scen: 'grandmasters', rank: '0.20,0.70,0.10', targets: [{ group: 20049, guaranteed: 0 }, { group: 20046, guaranteed: 0 }, { group: 20159, guaranteed: 0 }] }, global: {}, gm: {} })); } catch {} } });
    check('Grand Masters saved, Global data active: Unity Cup is used (its golds and rank)', (await p3.getAttribute('#x-scen', 'data-value')) === 'unity'
      && await p3.evaluate(() => window.__runs.at(-1).scenario === 'unity') && (await p3.getAttribute('#rank', 'data-value')) === '0.50,0.45,0.05' && (await p3.getAttribute('#g-20049', 'data-g')) === '2');
    await p3.context().close();
    check('v1 settings: R cards reset to off, other settings kept', !(await p2.locator('#x-rcards').isChecked()) && (await p2.getAttribute('#rank', 'data-value')) === '0.20,0.70,0.10');
    await p2.context().close();
  },
  async about(b) {
    // "How this works": a link in the header to its own page (/about), which links back
    const pg = await open(b);
    await pg.locator('header.top .tnav a', { hasText: 'How this works' }).click(); await pg.waitForLoadState();
    check('header link opens /about', new globalThis.URL(pg.url()).pathname === '/about' && /How this works/i.test(await pg.locator('main.about h2').innerText()));
    check('/about carries the model text and the version', /Model\./.test(await pg.locator('main.about').innerText()) && /Version v?\S+/.test(await pg.locator('main.about').innerText()));
    if (b.browserType().name() === 'chromium') { const f = await pg.evaluate(AUDIT); check('/about: every text run meets WCAG AAA', !f.length, f.slice(0, 5).join(' | ')); }
    await pg.locator('header.top .tnav a').click(); await pg.waitForSelector('#targets');
    check('/about links back to the planner', new globalThis.URL(pg.url()).pathname === '/');
    await pg.context().close();
  },
  async contrast(b) {
    if (b.browserType().name() !== 'chromium') return;   // one engine is enough for colours
    const pg = await open(b);
    await pg.locator('#roster .ut .up').nth(1).click(); await pg.locator('#cards .ct').nth(1).click(); await settle(pg);
    await pg.waitForTimeout(300);   // the results' fade back from the searching dim (0.15 s)
    const fails = [...await pg.evaluate(AUDIT)];
    await pg.click('#x-scen'); fails.push(...await pg.evaluate(AUDIT)); await pg.keyboard.press('Escape');
    await pg.locator('.deck .dc').first().click(); fails.push(...(await pg.evaluate(AUDIT)).filter(x => /cpop|sw|gtl|ctt/.test(x))); await pg.keyboard.press('Escape');
    await pg.locator('.loop .lacts button', { hasText: 'Race schedule' }).first().click(); await pg.waitForTimeout(200); fails.push(...await pg.evaluate(AUDIT));
    await pg.locator('#sched-dlg .stab').nth(4).click(); fails.push(...await pg.evaluate(AUDIT)); await pg.keyboard.press('Escape');
    await pg.locator('.loop .lacts button', { hasText: 'Affinity' }).first().click(); await pg.waitForTimeout(200); fails.push(...await pg.evaluate(AUDIT));
    check('contrast: every text run meets WCAG AAA', !fails.length, fails.slice(0, 5).join(' | '));
    await pg.context().close();
    const p2 = await b.newContext().then(c => c.newPage()); await p2.route('**/img.*/scen/**', r => r.abort()); await p2.goto(URL); await p2.waitForTimeout(800);
    await p2.click('#x-scen'); const bf = (await p2.evaluate(AUDIT)).filter(x => /^b\./.test(x) || /scen|stile|slogo|snm|ssub/.test(x));
    check('contrast: scenario badges (no logo files) meet WCAG AAA', !bf.length, bf.join(' | '));
    await p2.context().close();
  },
};

const pick = Object.keys(TESTS).filter(t => !argv.some(a => a in TESTS) || argv.includes(t));
const JOBS = Math.max(1, +process.env.LLP_JOBS || 4);
await Promise.all(BROWSERS.map(async (name) => {
  const b = await pw[name].launch(), queue = [...pick];
  const worker = async () => { for (let t; (t = queue.shift());) {
    const buf = [], t0 = Date.now();
    await LOG.run(buf, async () => { try { await TESTS[t](b); } catch (e) { failed++; say('  FAIL ' + t + ' threw: ' + String(e).split('\n')[0]); } });
    console.log(`[${name}] ${t} (${((Date.now() - t0) / 1000).toFixed(1)} s)\n` + buf.join('\n'));
  } };
  await Promise.all(Array.from({ length: JOBS }, worker));
  await b.close();
}));
console.log(`\n${passed} passed, ${failed} failed`);
process.exit(failed ? 1 : 0);
