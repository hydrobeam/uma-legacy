// Browser regression suite for the built page (webapp/dist, served on a local port; LLP_URL tests another address).
// Node + Playwright (local or global install). LLP_SHOTS=dir saves pictures of the scenario button and picker there.
//   node webapp/nodebrowser.mjs                 every test, Chromium and Firefox
//   node webapp/nodebrowser.mjs chromium roster only the named browsers / tests (any order)
// Both browsers run at once, each with LLP_JOBS test groups in parallel (default 4; LLP_JOBS=1 runs them one by one);
// a group's lines print together when it ends. Exit code 1 if any check fails. Expectations come from the page's own embedded data where possible, so a data
// update does not break the suite; the one fixed number is the Global default search (1 in 119, R cards off, Unity Cup).
import { createRequire } from 'module';
import { execSync } from 'child_process';
import { AsyncLocalStorage } from 'async_hooks';
import { serveDist } from './serve.mjs';

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
// before(pg): runs before the page loads (routes, request listeners)
async function open(browser, { w = 1440, h = 900, mobile = false, init, before, scheme = 'light' } = {}) {
  const ctx = await browser.newContext({ viewport: { width: w, height: h }, colorScheme: scheme, ...(mobile && browser.browserType().name() === 'chromium' ? { isMobile: true, hasTouch: true } : {}) });
  await ctx.addInitScript(() => {   // count worker creations/terminations
    const W = window.Worker; window.__made = 0; window.__killed = 0;
    window.__runs = [];   // the jobs sent to the engine, in order
    window.Worker = function (...a) { const w = new W(...a); window.__made++; const t = w.terminate.bind(w); w.terminate = () => { window.__killed++; t(); };
      const pm = w.postMessage.bind(w); w.postMessage = (m, ...r) => { if (m && m.type === 'run') window.__runs.push(m.job); return pm(m, ...r); }; return w; };
  });
  if (init) await ctx.addInitScript(init);
  const pg = await ctx.newPage(); pg.errors = [];
  pg.on('pageerror', e => pg.errors.push(String(e))); pg.on('console', m => m.type() === 'error' && !/downloadable font/.test(m.text()) && pg.errors.push(m.text()));   // web-font fetch aborts are network noise
  if (before) await before(pg);
  await pg.goto(URL); await settle(pg);
  return pg;
}
// a click that, when it times out, says what covers the target or whether it is missing (for intermittent failures)
async function clickSay(pg, loc, label) {
  try { await loc.click({ timeout: 15000 }); }
  catch (e) {
    const why = await loc.count() ? await loc.evaluate(el => { const r = el.getBoundingClientRect(), top = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
      return `covered by ${top ? top.tagName.toLowerCase() + '.' + top.className : 'nothing'} at ${Math.round(r.left)},${Math.round(r.top)}; busy ${document.getElementById('run-note').classList.contains('busy')}`; }) : 'not on the page';
    throw new Error(`click on ${label} timed out: ${why}`);
  }
}
const busy = (pg) => pg.evaluate(() => document.getElementById('run-note').classList.contains('busy'));
// wait for the current (auto) search to finish. A setting change marks the status line busy at once (the search
// itself starts after a short pause), so no fixed sleep is needed; after a page load, the page's first job must have
// been sent (pages opened without open()'s worker hook have no __runs and skip that part)
async function settle(pg, t = 180000) {
  await pg.waitForFunction(() => { const n = document.getElementById('run-note');
    return (window.__runs === undefined || window.__runs.length > 0) && !n.classList.contains('busy') && /Searched|example/i.test(n.textContent); }, null, { timeout: t });
}
const jobSent = (pg, fn, arg) => pg.waitForFunction(fn, arg);   // a search's job has gone to the engine (its result is not awaited)
const lastJob = (pg, scen) => jobSent(pg, (s) => window.__runs.length > 0 && window.__runs.at(-1).scenario === s, scen);
const kind = (pg, k) => pg.waitForFunction((k) => document.getElementById('rhead').dataset.kind === k, k);   // the status line's state
const dialog = (pg, id) => pg.waitForSelector(`#${id}[open]`);
// a condition the page reaches on its own (an observer, a smooth scroll, a resize handler): true once it holds, false
// if it never does within the time
const until = (pg, fn, arg, timeout = 3000) => pg.waitForFunction(fn, arg, { timeout }).then(() => true, () => false);
async function navClick(pg, sel) { await Promise.all([pg.waitForEvent('load'), pg.click(sel)]); }   // a click that reloads the page
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
    if (el.closest('h1.brand') && /^b[uma]$/.test(el.className)) continue;   // the brand's coloured "uma": the logo's colours by choice
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
    check('… and its status says what the result was checked against', /Checked on exact odds|Ranked by odds with targets treated as independent/.test(await note(pg)), await note(pg));
    // the result's wording: the headline odds, the "at least 7 of 8" line, each member's own odds as trainee
    const head = await pg.evaluate(() => { const l = document.querySelector('#results .loop'); return { odds: l.querySelector('.odds').innerText, sub: l.querySelector('.sub').innerText, rank: !!l.querySelector('.rank'),
      own: [...l.querySelectorAll('.mem .own')].map(e => e.innerText), font: getComputedStyle(l.querySelector('.odds')).fontFamily }; });
    check('headline: "1 in 107 runs to land all 8", then "at least 7 of 8" and the 90% line, in the UI font; no rank with one result',
      /^1 in 107 runs to land all 8$/.test(head.odds) && /^7 of 8 or better: 1 in 12\.6 · a 90% chance within 245 runs$/.test(head.sub) && !head.rank && /Zen Kaku/.test(head.font), JSON.stringify(head));
    // the default result is a three-uma rotation: three members in three columns, a deck for each
    check('members: three (a rotation), "as trainee: 1 in …", in three columns', head.own.length === 3 && head.own.every(x => /^as trainee: 1 in [\d,.]+$/.test(x))
      && await pg.evaluate(() => { const m = document.querySelector('#results .loop .members'); return m.classList.contains('n3') && getComputedStyle(m).gridTemplateColumns.split(' ').length === 3; }), head.own.join(' | '));
    // a pal whose credited event choice needs a priority says so, derived from the event data (Anshinzawa SSR's finale)
    const anshin = await pg.evaluate(() => [...document.querySelectorAll('#results .loop .dc[data-cid="30080"] i')].map(i => i.textContent));
    check('deck: Sasami Anshinzawa SSR says "prioritise Risky Business"', anshin.includes('prioritise Risky Business'), JSON.stringify(anshin));
    // the deck's odds per target, each with its skill icon
    const dd = await pg.evaluate(() => { const l = document.querySelector('#results .loop .dodds'); return { n: l.querySelectorAll('.dsk').length, icons: l.querySelectorAll('.dsk img.sic').length }; });
    check('deck odds: a skill icon on every entry', dd.n > 0 && dd.icons === dd.n, JSON.stringify(dd));
    // the odds sit in a frame the (i) button opens; a swap row names its uma by her character icon
    const oi = pg.locator('#results .loop .deck .oinfo').first();
    check('deck odds start closed', await pg.evaluate(() => document.querySelector('#results .loop .dodds').hidden));
    await oi.click();
    check('(i) opens the deck odds (aria-expanded, names and percentages)', (await oi.getAttribute('aria-expanded')) === 'true' && await pg.evaluate(() => { const o = document.querySelector('#results .loop .dodds'); return !o.hidden && o.querySelectorAll('.dnm').length > 0 && /\d+%/.test(o.innerText); }));
    await oi.click();
    check('(i) closes them again', await pg.evaluate(() => document.querySelector('#results .loop .dodds').hidden));
    const sw = await pg.evaluate(() => [...document.querySelectorAll('#results .loop .deck.swap .swho')].map(w => ({ av: w.querySelectorAll('.av').length, label: w.querySelector('.vh').textContent, tips: [...w.querySelectorAll('.facebtn')].map(a => a.title), shown: [...w.childNodes].filter(n => !(n.classList && (n.classList.contains('vh') || n.classList.contains('facebtn')))).map(n => n.textContent).join('').trim() })));   // .vh: screen-reader text
    const swd = await pg.evaluate(() => [...document.querySelectorAll('#results .loop .deck.swap')].map(d => [...d.querySelectorAll('.swd1')].map(x => x.textContent.trim())));
    check('each swap row says what it changes ("Skill a% → b%")', swd.length > 0 && swd.every(r => r.length > 0 && r.every(t => /\d+% → \d+%$/.test(t))), JSON.stringify(swd));
    const more = pg.locator('#results .loop .swd.more').first();
    if (await more.count()) { const n0 = await more.locator('.swd1:visible').count(); await more.click();
      check('clicking a swap\'s changes shows them all, again folds them', (await more.locator('.swd1:visible').count()) > n0 && (await more.getAttribute('aria-expanded')) === 'true'
        && (await more.click(), (await more.locator('.swd1:visible').count()) === n0)); }
    check('a label marks where the swap rows start', await pg.evaluate(() => { const h = document.querySelector('#results .loop .swhead'), s = document.querySelector('#results .loop .deck.swap'); return !!h && /swaps/i.test(h.textContent) && !!(h.compareDocumentPosition(s) & Node.DOCUMENT_POSITION_FOLLOWING); }));
    check('swap rows show the uma as a character icon, named for screen readers', sw.length > 0 && sw.every(x => x.av > 0 && (x.av === 1 ? /deck/ : / and /).test(x.label) && x.tips.length === x.av && x.tips.every(t => t && !/ and |’s deck/.test(t)) && x.shown === ''), JSON.stringify(sw));
    const al = await pg.evaluate(() => Math.round(document.querySelector('#results .loop').getBoundingClientRect().top - document.querySelector('.rail .panel').getBoundingClientRect().top));
    check('the first result is level with the Settings box', Math.abs(al) <= 1, al + ' px');
    check('the Settings box holds the status line, "Results to show" and the lineage rank', await pg.evaluate(() => { const s = document.querySelector('section[aria-labelledby="h-era"]');
      return !!s && /Settings/.test(document.getElementById('h-era').textContent) && ['run-note', 'c-top', 'rank'].every(id => s.contains(document.getElementById(id))); }));
    // the results stay in view while the settings column scrolls (sticky column, scrolling on its own)
    await pg.evaluate(() => scrollTo(0, 1600));
    const st = await pg.evaluate(() => { const c = document.getElementById('rcol').getBoundingClientRect(), l = document.querySelector('#results .loop').getBoundingClientRect();
      return { top: Math.round(c.top), bottom: Math.round(c.bottom), loop: Math.round(l.top), y: scrollY, h: innerHeight }; });
    check('desktop: scrolled down the settings, the result stays on screen', st.y > 1000 && st.top >= 0 && st.top <= 20 && st.loop < st.h && st.bottom <= st.h, JSON.stringify(st));
    await pg.evaluate(() => scrollTo(0, 0));
    // the scenario sets the training-hint rate: switching it searches again with the scenario in the job, and the
    // choice is kept (the default setup's odds barely move: its decks get the targets from events, not training hints)
    const n0 = await pg.evaluate(() => window.__runs.length);
    check('the first search is a Unity Cup job', await pg.evaluate(() => window.__runs.at(-1).scenario === 'unity'));
    await pickScen(pg, 'ura'); await lastJob(pg, 'ura');   // the check reads the job: its (slow) search is not awaited
    check('scenario switch searches again with the URA job', await pg.evaluate((n) => window.__runs.length > n && window.__runs.at(-1).scenario === 'ura', n0));
    // Unity Cup's golds follow the scenario: off under URA, back on under Unity, and the 8/8 example takes them too
    check('URA drops the Unity golds', (await pg.getAttribute('#g-20049', 'data-g')) === '0' && (await pg.getAttribute('#g-20046', 'data-g')) === '0');
    await pg.click('#t-preset');
    check('8/8 example under URA: eight targets, no guaranteed golds', (await pg.locator('#targets .tgt').count()) === 8 && (await pg.getAttribute('#g-20049', 'data-g')) === '0');
    await pg.reload(); await jobSent(pg, () => window.__runs.length > 0);
    check('scenario survives a reload', (await scen(pg)) === 'ura' && (await pg.getAttribute('#x-scen', 'aria-label')) === 'Scenario: URA Finale');
    await pickScen(pg, 'unity');
    check('Unity Cup marks No Stopping Me! and It\'s On! as guaranteed golds', (await pg.getAttribute('#g-20049', 'data-g')) === '2' && (await pg.getAttribute('#g-20046', 'data-g')) === '2');
    await settle(pg);
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async scenario(b) {
    // the scenario picker: a button showing the chosen scenario's logo, opening a grid of the scenarios
    const pg = await open(b);
    await pg.click('#rank-q');
    check('lineage rank: the ? opens its explanation', !(await pg.locator('#rank-note').isHidden()) && (await pg.getAttribute('#rank-q', 'aria-expanded')) === 'true');
    await pg.click('#rank-q');
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
      return { v: t.dataset.v, sub: sub.innerText.replace(/\s+/g, ' ').trim(), w: Math.round(r.width), h: Math.round(r.height), gap: Math.round(r.bottom - pad - sr.bottom), subH: Math.round(sr.height) }; }));
    // hint rate as a multiple of URA Finale's (2.70, 2.03, 1.60, 1.81 hints per card over 1.60)
    const want = { unity: 'Hint rate ×1.69', gc: 'Hint rate ×1.27', ura: 'Hint rate ×1.00', trackblazer: 'Hint rate ×1.13' };
    check('each tile has the same hover text as the button gives the chosen scenario', await pg.evaluate(() => [...document.querySelectorAll('#scen-menu .stile')].every(t => t.title && t.title === t.getAttribute('aria-label')) && document.querySelector('#scen-menu .stile[data-v="unity"]').title === document.getElementById('x-scen').title));
    check('opens with four tiles, each with its hint rate against URA Finale', !(await menu.isHidden()) && t.length === 4 && t.every(x => x.sub === want[x.v]), t.map(x => x.v + ': ' + x.sub).join(' | '));
    check('the four tiles are the same size', new Set(t.map(x => x.w + 'x' + x.h)).size === 1, t.map(x => x.w + 'x' + x.h).join(' '));
    check('every tile is logo + one line, no blank space below', t.every(x => x.subH <= 22 && x.gap <= 1), t.map(x => `${x.v} line ${x.subH}px, blank ${x.gap}px`).join(' | '));
    // Unity Cup's guaranteed golds: their gold skill icons on the hint-rate line, named by tooltip and alt text
    const golds = await pg.evaluate(() => { const skl = JSON.parse(document.getElementById('data-skills').textContent), gi = JSON.parse(document.getElementById('data-skillicon').textContent).gold;
      const g = document.querySelector('#scen-menu .stile[data-v="unity"] .ssub .sgolds');
      return { want: [20049, 20046].map(x => ({ name: skl.find(s => s[0] === x)[2], src: `/skill/${gi[x]}.avif` })), group: g && g.getAttribute('aria-label'),
        got: g ? [...g.querySelectorAll('img')].map(i => ({ alt: i.alt, title: i.title, src: i.getAttribute('src'), h: Math.round(i.getBoundingClientRect().height) })) : [],
        text: g ? g.innerText.trim() : null, others: document.querySelectorAll('#scen-menu .stile:not([data-v="unity"]) .sgolds').length }; });
    check('Unity Cup tile: No Stopping Me! and It\'s On! as gold icons on the rate line, no visible names', golds.group === 'Guaranteed in every run' && golds.others === 0 && golds.text === ''
      && golds.got.length === 2 && golds.want.every((w, i) => golds.got[i].alt === w.name && golds.got[i].title === w.name && golds.got[i].src.endsWith(w.src) && golds.got[i].h <= 18), JSON.stringify(golds.got));
    check('logos keep their aspect (object-fit: contain)', await pg.evaluate(() => [...document.querySelectorAll('#scen-dd .slogo img')].every(i => i.naturalWidth > 0 && getComputedStyle(i).objectFit === 'contain')));
    check('the open picker marks the chosen tile and focuses it', await pg.evaluate(() => { const t = document.activeElement; return t.classList.contains('stile') && t.dataset.v === 'unity' && t.getAttribute('aria-checked') === 'true'; })
      && (await pg.getAttribute('#x-scen', 'aria-expanded')) === 'true');
    if (shots) await menu.screenshot({ path: `${shots}/${bn}-scen-picker.png` });
    const r0 = await pg.evaluate(() => window.__runs.length);
    await tiles.nth(0).click();   // a change would show "Searching…" at once
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
    await pg.click('#x-scen'); await pg.keyboard.press('End'); await pg.keyboard.press('Enter'); await lastJob(pg, 'trackblazer');
    check('Enter on a tile picks it and closes the picker', (await menu.isHidden()) && (await scen(pg)) === 'trackblazer' && (await btnName()) === 'Scenario: Trackblazer');
    check('… and searches with it', await pg.evaluate(() => window.__runs.at(-1).scenario === 'trackblazer'));
    // the lineage rank follows the scenario: Grand Concert SS, Unity Cup below SS
    await pickScen(pg, 'gc'); await lastJob(pg, 'gc');
    const sirius = await pg.evaluate(() => { const t = [...document.querySelectorAll('#cards .ct')].find(x => /Team Sirius/.test(x.textContent)); return t ? (t.querySelector('.cban') || {}).textContent || '' : null; });
    check('Team Sirius: "Not allowed in Grand Concert" in the card picker', sirius === 'Not allowed in Grand Concert', String(sirius));
    check('picking Grand Concert sets the rank to SS', (await pg.getAttribute('#rank', 'data-value')) === '0.20,0.70,0.10' && await pg.evaluate(() => window.__runs.at(-1).star_dist.join() === '0.2,0.7,0.1'));
    await pickScen(pg, 'unity'); await settle(pg);
    check('picking Unity Cup sets it back to below SS', (await pg.getAttribute('#rank', 'data-value')) === '0.50,0.45,0.05');
    await pg.click('#rank button[data-v="0.20,0.70,0.10"]'); await settle(pg); await pickScen(pg, 'unity');
    check('a rank changed by hand sticks (re-picking the scenario leaves it)', (await pg.getAttribute('#rank', 'data-value')) === '0.20,0.70,0.10' && await pg.evaluate(() => window.__runs.at(-1).star_dist.join() === '0.2,0.7,0.1'));
    await pg.reload(); await settle(pg);
    check('… and survives a reload', (await pg.getAttribute('#rank', 'data-value')) === '0.20,0.70,0.10' && (await scen(pg)) === 'unity');
    // the rank toggle is a radio group: game badges, arrow keys move the choice
    await pg.focus('#rank button[aria-checked="true"]'); await pg.keyboard.press('ArrowLeft');
    check('lineage rank: arrow keys switch it, aria-checked follows, badges shown', (await pg.getAttribute('#rank', 'data-value')) === '0.50,0.45,0.05'
      && (await pg.getAttribute('#rank button[data-v="0.50,0.45,0.05"]', 'aria-checked')) === 'true' && (await pg.locator('#rank .rk.img img').count()) === 3);
    await pg.keyboard.press('ArrowRight'); await settle(pg);
    await pg.click('#ex-open');
    check('export names the scenario', JSON.parse(await pg.inputValue('#ex-text')).readable.scenario === 'Unity Cup');
    const text = (await pg.inputValue('#ex-text')).replace('"scen": "unity"', '"scen": "gc"');
    await pg.fill('#ex-text', text); await pg.click('#ex-load'); await pg.click('#ex-close');
    check('import updates the button', (await scen(pg)) === 'gc' && (await btnName()) === 'Scenario: Grand Concert');   // its (slow) search is not awaited
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
    // without the logo files (images not built, or a failed fetch) the button and tiles show the name on a badge
    const p2 = await open(b, { before: (p) => p.route('**/img.*/scen/**', r => r.abort()) });
    await until(p2, () => !document.querySelector('#scen-dd .slogo img')); await p2.click('#x-scen');
    const badges = await p2.evaluate(() => [...document.querySelectorAll('#scen-dd .slogo')].map(l => (l.querySelector('img') ? 'img' : '') + (l.querySelector('b') || {}).textContent).join(','));
    check('missing logos fall back to name badges', badges === 'Unity Cup,Unity Cup,Grand Concert,URA Finale,Trackblazer', badges);
    const sz = await p2.evaluate(() => new Set([...document.querySelectorAll('#scen-menu .stile')].map(t => { const r = t.getBoundingClientRect(); return Math.round(r.width) + 'x' + Math.round(r.height); })).size);
    check('… and the tiles stay the same size', sz === 1);
    check('Grand Masters is not offered with the Global data', (await p2.locator('#scen-menu .stile').count()) === 4 && (await p2.locator('#scen-menu .stile[data-v="grandmasters"]').count()) === 0);
    await p2.context().close();
  },
  async autosearch(b) {
    const pg = await open(b);
    // auto-search off: a change waits for the Search button; back on, it searches
    check('auto-search is on by default', await pg.locator('#x-auto').isChecked());
    await pg.click('#x-auto'); const n0 = await pg.evaluate(() => window.__runs.length);
    check('switching auto-search off leaves the results as they are (not out of date)', /Searched/.test(await note(pg)) && !(await pg.evaluate(() => document.getElementById('results').classList.contains('stale'))));
    await pg.locator('#cards .ct').nth(6).click(); await pg.waitForTimeout(1300);
    check('auto-search off: a change does not search, the status says to press Search', (await pg.evaluate(() => window.__runs.length)) === n0 && /press Search/.test(await note(pg)) && (await pg.locator('#r-act button', { hasText: 'Search' }).count()) === 1);
    await pg.locator('#r-act button', { hasText: 'Search' }).click(); await settle(pg);
    check('… the Search button runs it', (await pg.evaluate(() => window.__runs.length)) === n0 + 1 && /Searched/.test(await note(pg)));
    await pg.locator('#cards .ct').nth(6).click(); await pg.click('#x-auto'); await settle(pg);
    check('… turning it back on searches the change', (await pg.evaluate(() => window.__runs.length)) === n0 + 2);
    const made0 = await pg.evaluate(() => window.__made);
    await pg.evaluate(() => [...document.querySelectorAll('#cards .ct')].slice(0, 5).forEach(b => b.click()));   // five changes inside one pause
    check('a change shows "Searching…"', /Searching/.test(await note(pg)));
    // the fast pass is drawn first, with the status saying the exact check is still running
    const checking = await pg.waitForFunction(() => /Checking the exact odds/.test(document.getElementById('run-note').textContent), null, { timeout: 30000 }).then(() => true, () => false);
    check('the first pass shows while the exact check runs', checking && (await pg.locator('#results .loop').count()) > 0, await note(pg));
    await settle(pg);
    check('a burst of 5 changes is one search on the same worker', (await pg.evaluate(() => window.__made)) === made0);
    // a slow search (3 results, one shared deck), changed mid-run: the worker is killed and the new search finishes
    const r0 = await pg.evaluate(() => window.__runs.length);
    await pg.selectOption('#c-top', '3'); await pg.click('#c-peruma');
    await pg.waitForFunction((n) => window.__runs.length > n && window.__runs.at(-1).top_decks === 3 && window.__runs.at(-1).per_uma === false, r0);   // the slow search itself is running (not a quick job from the first change alone)
    const k0 = await pg.evaluate(() => window.__killed);
    check('old results stay visible while searching', (await pg.locator('#results .loop').count()) > 0);
    await pg.waitForFunction(() => /(Searching|Checking the exact odds)…\s*\d+ s/.test(document.getElementById('run-note').textContent), null, { timeout: 30000 });   // its clock is running
    await pg.click('#c-peruma'); const paused = await note(pg);
    check('a change mid-search clears the clock: it counts again only once the new search runs', /Searching/.test(paused) && !/\d+ s/.test(paused), paused);
    await pg.selectOption('#c-top', '1'); await settle(pg);
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
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async cards(b) {
    const pg = await open(b);
    const secs = await pg.evaluate(() => [...document.querySelectorAll('#cards .cgroup h3')].map(h => h.textContent));
    check('card sections SSR, SR, R in that order', secs.join(',') === 'SSR,SR,R', secs.join(','));
    const notes = await pg.evaluate(() => [...document.querySelectorAll('#cards .gv')].map(g => [g.querySelector('small').textContent, g.title]));
    check('card hint notes in plain words, each with a tooltip', notes.length > 0 && notes.every(([n, t]) => /^(hint|event|\d(st|nd|rd|th) event)( · \d+%)?$/.test(n) && t.length > 0), JSON.stringify(notes.filter(([n]) => !/^(hint|event|\d(st|nd|rd|th) event)( · \d+%)?$/.test(n)).slice(0, 3)));
    const art = await pg.evaluate(() => { const s = new Set(); let over = 0; for (const t of document.querySelectorAll('#cards .ct')) { const c = t.querySelector('.cav').getBoundingClientRect();
      s.add(Math.round(c.width)); if (c.width > t.getBoundingClientRect().width + 0.5) over++; } return { sizes: [...s], over }; });
    check('card list: every card picture is the same size and inside its tile, loaded or not', art.sizes.length === 1 && !art.over, JSON.stringify(art));
    check('card names carry no rarity (the frame and section show it)', await pg.evaluate(() => ![...document.querySelectorAll('#cards .ct .cn')].some(e => / (SSR|SR|R)$/.test(e.textContent))));
    // the game bans Team Sirius from Unity Cup and Grand Concert decks: the picker says so and no result deck holds it
    const sirius = () => pg.evaluate(() => { const t = [...document.querySelectorAll('#cards .ct')].find(x => /Team Sirius/.test(x.textContent)); return t ? (t.querySelector('.cban') || {}).textContent || '' : null; });
    check('Team Sirius: "Not allowed in Unity Cup" in the card picker', (await sirius()) === 'Not allowed in Unity Cup', String(await sirius()));
    check('… and in no Unity Cup result deck', await pg.evaluate(() => ![...document.querySelectorAll('#results .dc')].some(d => d.dataset.cid === '30081')));
    await pickScen(pg, 'ura');   // the picker follows at once; the search is not awaited
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
    // R cards are left out (no setting): the R section holds only the R pals
    const rpals = await pg.evaluate(() => JSON.parse(document.getElementById('data-cards').textContent).filter(c => c.rar === 1 && c.pal).length);
    const rCount = () => pg.evaluate(() => { const g = [...document.querySelectorAll('#cards .cgroup')].find(x => x.querySelector('h3').textContent === 'R'); return g ? g.querySelectorAll('.ct').length : 0; });
    check('R cards are left out, with no switch; the R section keeps the R pals', (await pg.locator('#x-rcards').count()) === 0 && (await rCount()) === rpals, (await rCount()) + ' of ' + rpals);
    check('every card image carries its type icon', (await pg.locator('#cards .ct .cav.art').count()) === (await pg.locator('#cards .ct .ctype').count()));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async panels(b) {
    const pg = await open(b);
    const dc = pg.locator('.deck .dc').first(); await dc.click();
    const href = await pg.locator('.cpop .gtl').getAttribute('href');
    check('deck card panel: GameTora card link', /^https:\/\/gametora\.com\/umamusume\/supports\/\d+-[a-z0-9-]+$/.test(href || ''), href);
    const side = await pg.evaluate(() => { const c = document.querySelector('.deck .dc').getBoundingClientRect(), p = document.querySelector('.cpop').getBoundingClientRect(); return { beside: p.left >= c.right || p.right <= c.left, top: Math.abs(p.top - c.top) }; });
    check('deck card panel opens beside the card (desktop), not over the deck below', side.beside && side.top <= 2, JSON.stringify(side));
    const cid = await dc.getAttribute('data-cid');
    await pg.locator('.cpop .sw').click();
    check('deck card Owned switch updates the card picker', await pg.evaluate((id) => [...document.querySelectorAll('#cards .ct')].some(t => t.querySelector('img') && t.getAttribute('aria-pressed') === 'false'), cid));
    check('… and greys the card in the result', (await pg.locator('.deck .dc.off').count()) > 0);
    await pg.locator('.cpop .sw').click(); await pg.keyboard.press('Escape');
    check('Escape closes the panel', await pg.evaluate(() => document.querySelector('.cpop').hidden));
    // an outfit skill under a member carries its skill icon
    const sp = pg.locator('.loop').first().locator('.mem .spark').first();
    check('outfit skill shows its skill icon', (await sp.locator('img.sic').count()) === 1);
    const mb = pg.locator('.loop').first().locator('.mem .memb').nth(1); await mb.click();
    await pg.keyboard.press('Escape');
    const fb = pg.locator('.loop').first().locator('.swho .facebtn').first();
    if (await fb.count()) { const id = await fb.getAttribute('data-uma'); await fb.click();
      check('a portrait in a swap row opens the same member panel', !(await pg.locator('.cpop').isHidden()) && (await pg.locator('.cpop input[data-act="uma"]').count()) === 1
        && (await pg.locator('.cpop b').first().innerText()) === await pg.locator(`.loop .mem[data-uma="${id}"] .nm`).first().innerText());
      await pg.keyboard.press('Escape'); await mb.click(); }
    check('member panel has one Owned switch', (await pg.locator('.cpop input[role=switch]').count()) === 1);
    const uh = await pg.locator('.cpop .gtl').getAttribute('href');
    check('member panel: GameTora character link', /^https:\/\/gametora\.com\/umamusume\/characters\/\d{6}-[a-z0-9-]+$/.test(uh || ''), uh);
    await pg.locator('.cpop .sw').click();
    check('member Owned switch updates the roster and greys her', (await pg.locator('#roster .ut[data-s="out"]').count()) === 1 && (await pg.locator('.mem.off').count()) > 0);
    await pg.locator('.cpop .sw').click(); await pg.keyboard.press('Escape'); await settle(pg);
    // must include, then Owned off in her panel: she is not owned and no longer must include (no contradiction left)
    const m0 = pg.locator('.loop').first().locator('.mem').first(), uid = await m0.getAttribute('data-uma'), anyOutfit = !(await m0.getAttribute('data-outfit'));
    const tile = pg.locator(`#roster .ut[data-uma="${uid}"] .up`).first(); await clickSay(pg, tile, 'her roster tile'); await clickSay(pg, tile, 'her roster tile again'); await settle(pg);
    const marked = await pg.locator(`#roster .ut[data-uma="${uid}"][data-s="req"]`).count();
    const ring = await pg.evaluate((id) => { const c = document.querySelector('.loop'), f = [...c.querySelectorAll('.mem.forced')].map(m => m.dataset.uma);
      return f.length === 1 && f[0] === id && /Must include/.test((c.querySelector('.lhead .lkey') || {}).textContent || '')
        && getComputedStyle(c.querySelector('.mem.forced .av')).boxShadow !== 'none' && /must include/.test(c.querySelector('.mem.forced .memb').textContent); }, uid);
    check('a must-include uma has the green ring in the result, with the legend at the top right', ring);
    await clickSay(pg, pg.locator('.loop').first().locator(`.mem[data-uma="${uid}"] .memb`), 'her name in the result'); await clickSay(pg, pg.locator('.cpop .sw'), 'the Owned switch'); await pg.keyboard.press('Escape'); await settle(pg);
    const after = await pg.evaluate((id) => [...document.querySelectorAll(`#roster .ut[data-uma="${id}"]`)].map(t => t.dataset.s), uid);
    // (her first tile's outfit is the one marked, so the loop uses it and the switch turns that outfit off; with "any
    // outfit" every tile of hers goes off)
    check('must include, then Owned off in the member panel: not owned, no longer must include, and the search runs', marked === 1 && after[0] === 'out' && !after.includes('req')
      && (!anyOutfit || after.every(x => x === 'out')) && /Searched/.test(await note(pg)), after.join(',') + ' · ' + await note(pg));
    check('… and with nothing required, no ring and no legend', !(await pg.locator('.mem.forced, .lhead .lkey').count()));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async targets(b) {
    // the target list by keyboard: removing keeps the focus in the list, the add box takes Enter and the arrows
    const pg = await open(b);
    const skl = await json(pg, 'data-skills'), gold = (g) => skl.find(x => x[0] === g)[2];
    // the add box sits above the list, two targets per row
    const lay = await pg.evaluate(() => { const t = [...document.querySelectorAll('#targets .tgt')].map(e => e.getBoundingClientRect());
      return { rows: new Set(t.map(r => Math.round(r.top))).size, n: t.length, addFirst: document.getElementById('t-add').getBoundingClientRect().bottom <= Math.min(...t.map(r => r.top)) }; });
    check('targets: add box first, two per row', lay.addFirst && lay.rows === Math.ceil(lay.n / 2), JSON.stringify(lay));
    check('a gold the scenario guarantees shows as the gold itself, in gold', (await pg.locator('#g-20049 .name').innerText()).trim() === gold(20049) && await pg.locator('#g-20049').evaluate(e => e.classList.contains('gold')), await pg.locator('#g-20049').innerText());
    await pg.locator('#targets .x').nth(2).focus(); await pg.keyboard.press('Enter');
    check('removing a target by keyboard moves the focus to the next remove button', (await pg.locator('#targets .tgt').count()) === 7
      && await pg.evaluate(() => document.activeElement.classList.contains('x') && [...document.querySelectorAll('#targets .x')].indexOf(document.activeElement) === 2));
    for (let i = 0; i < 7; i++) await pg.keyboard.press('Enter');
    check('… and to the add box after the last one', (await pg.locator('#targets .tgt').count()) === 0 && await pg.evaluate(() => document.activeElement.id === 't-add'));
    await pg.fill('#t-add', 'corner');
    const first = await pg.locator('#t-sugg button').first().innerText();
    await pg.keyboard.press('ArrowDown'); await pg.keyboard.press('ArrowDown'); await pg.keyboard.press('ArrowUp');
    const at = await pg.evaluate(() => [...document.querySelectorAll('#t-sugg button')].indexOf(document.activeElement));
    await pg.keyboard.press('ArrowUp'); const back = await pg.evaluate(() => document.activeElement.id);
    check('arrows walk the suggestions, Up from the first returns to the box', at === 0 && back === 't-add', at + ' ' + back);
    await pg.keyboard.press('ArrowDown'); await pg.keyboard.press('Escape');
    check('Escape on a suggestion closes the list and returns to the box', await pg.locator('#t-sugg').isHidden() && await pg.evaluate(() => document.activeElement.id === 't-add'));
    await pg.fill('#t-add', 'corner'); await pg.keyboard.press('Enter');
    check('Enter adds the first match and keeps the focus in the box', (await pg.locator('#targets .tgt').count()) === 1 && (await pg.locator('#targets .tgt').innerText()).includes(first.split(' → ')[0]) && await pg.evaluate(() => document.activeElement.id === 't-add'), first);
    await pg.fill('#t-add', first.split(' → ')[0].slice(0, 8));
    check('a skill already listed: "… is already a target"', (await pg.locator('#t-sugg .snote').allInnerTexts()).some(t => t === first.split(' → ')[0] + ' is already a target'));
    await pg.fill('#t-add', 'zzqx');
    check('no match: "No skill matches"', /^No skill matches/.test(await pg.locator('#t-sugg').innerText()));
    // a gold matched by its own name shows with its white
    const g = skl.find(x => x[2] && !x[1].toLowerCase().includes(x[2].toLowerCase().slice(0, 6)) && !skl.some(y => y !== x && (y[1] + (y[2] || '')).toLowerCase().includes(x[2].toLowerCase())));
    await pg.fill('#t-add', g[2]);
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
    await pg.locator('.loop .lacts button', { hasText: 'Race schedule' }).first().click(); await dialog(pg, 'sched-dlg');
    await pg.locator('#sched-dlg .stab').first().focus(); await pg.keyboard.press('ArrowRight');
    const tabs = await pg.evaluate(() => [...document.querySelectorAll('#sched-dlg .stab')].map(t => [t.getAttribute('aria-selected'), t.tabIndex, t.getAttribute('aria-controls')].join(':')));
    check('schedule tabs: ArrowRight selects and focuses the next tab, one tab in the Tab order, each controls the panel', tabs[1] === 'true:0:sched-view' && tabs.filter(t => t.startsWith('true')).length === 1
      && await pg.evaluate(() => document.activeElement === document.querySelectorAll('#sched-dlg .stab')[1]), tabs.join(' '));
    await pg.keyboard.press('Escape');
    // affinity: each career is a button
    await pg.locator('.loop .lacts button', { hasText: 'Affinity' }).first().click(); await dialog(pg, 'why-dlg');
    const rb = pg.locator('#why-dlg .rowbtn[aria-pressed="false"]').first(); await rb.focus(); await pg.keyboard.press('Enter');
    check('affinity: the careers are buttons; Enter shows one', await pg.evaluate(() => document.activeElement.getAttribute('aria-pressed') === 'true' && document.activeElement.closest('tr').classList.contains('sel'))
      && /choose one/i.test(await pg.locator('#why-dlg').innerText()));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async states(b) {
    // what the status line says when the settings can't be searched, and the results on screen meanwhile (kept, dimmed)
    const pg = await open(b);
    const head = () => pg.evaluate(() => ({ kind: document.getElementById('rhead').dataset.kind, note: document.getElementById('run-note').textContent, act: document.getElementById('r-act').innerText,
      loops: document.querySelectorAll('#results .loop').length, stale: document.getElementById('results').classList.contains('stale') }));
    while (await pg.locator('#targets .x').count()) await pg.locator('#targets .x').first().click();
    await kind(pg, 'empty'); let h = await head();
    check('no targets: a neutral status with a button that loads the example targets; the old result stays, dimmed', h.kind === 'empty' && /Add a target skill/.test(h.note) && /8\/8 example/.test(h.act) && h.loops > 0 && h.stale, JSON.stringify(h));
    await pg.click('#r-act button'); await settle(pg); h = await head();
    check('… the button loads them and the search runs', (await pg.locator('#targets .tgt').count()) === 8 && h.kind === 'done' && /Searched/.test(h.note) && !h.stale, JSON.stringify(h));
    await pg.click('#r-none'); await kind(pg, 'empty'); h = await head();
    check('no umas owned: a neutral status with "Mark all umas owned"', h.kind === 'empty' && /at least 3 umas/.test(h.note) && /Mark all umas owned/.test(h.act) && h.loops > 0, JSON.stringify(h));
    await pg.click('#r-act button'); await settle(pg);
    // contradictory settings (an import can carry them): the status says what is wrong, the result stays
    const u = (await json(pg, 'data-umas')).find(x => x.outfits.length > 1);
    await pg.click('#ex-open'); const ex = JSON.parse(await pg.inputValue('#ex-text'));
    ex.settings.roster = { [u.id]: 'out' }; ex.settings.outfits = { [u.outfits[0].id]: 'req' };
    await pg.fill('#ex-text', JSON.stringify(ex)); await pg.click('#ex-load'); await pg.click('#ex-close'); await kind(pg, 'error'); h = await head();
    check('must include but not owned: an error naming her, in the status line; results kept', h.kind === 'error' && h.note.includes(u.name + ' is marked Must include but not owned') && h.loops > 0 && h.stale, JSON.stringify(h));
    if (b.browserType().name() === 'chromium') { const cf = (await pg.evaluate(AUDIT)).filter(x => /^p\.runnote/.test(x)); check('contrast: the error status line meets WCAG AAA', !cf.length, cf.join(' | ')); }
    await pg.click('#r-all'); await settle(pg);
    // Stop: a slow search (three results, one shared deck) is stopped; the results stay and the worker is killed;
    // "Search again" runs it to the end, and the three results are told apart
    const r0 = await pg.evaluate(() => window.__runs.length);
    await pg.selectOption('#c-top', '3'); await pg.click('#c-peruma');
    await pg.waitForFunction((n) => window.__runs.length > n && window.__runs.at(-1).top_decks === 3 && window.__runs.at(-1).per_uma === false, r0);   // the slow search itself is running (not a job from the first change alone)
    check('while searching: the results are marked busy and a Stop button shows', await pg.evaluate(() => document.getElementById('results').getAttribute('aria-busy') === 'true') && (await pg.locator('#r-act button').innerText()) === 'Stop');
    const k0 = await pg.evaluate(() => window.__killed); await pg.click('#r-act button'); h = await head();
    // stopped before the first pass is drawn: the old results stay, dimmed; stopped during the exact check: the first
    // pass stays, current, and the status says it was not checked
    const stoppedOk = h.kind === 'stopped' || (h.kind === 'done' && /Stopped the exact check/.test(h.note) && !h.stale);
    check('Stop kills the search and keeps the results', (await pg.evaluate(() => window.__killed)) > k0 && stoppedOk && h.loops > 0 && !(await busy(pg)), JSON.stringify(h));
    await pg.click('#r-act button'); await settle(pg);
    check('… and "Search again" runs it', /Searched/.test(await note(pg)) && (await pg.locator('#results .loop').count()) === 3);
    const r = await pg.evaluate(() => { const L = [...document.querySelectorAll('#results .loop')], key = (l) => [...l.querySelectorAll('.mem')].map(m => m.dataset.uma + ':' + m.dataset.outfit).join(' ');
      return L.map((l, i) => ({ rank: (l.querySelector('.rank') || {}).textContent, same: !!l.querySelector('.same'), mem: l.querySelectorAll('.mem').length,
        sameAs1: i > 0 && !l.querySelector('.same') && key(l) === key(L[0]) })); });
    check('three results: ranked #1, #2, #3', r.map(x => x.rank).join() === '#1,#2,#3', JSON.stringify(r));
    check('a result with #1\'s loop says "Same loop as #1" instead of repeating the umas', r.some(x => x.same) && r.every(x => x.same ? x.mem === 0 : x.mem === 3 || x.mem === 4) && !r.some(x => x.sameAs1), JSON.stringify(r));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  // three results per uma: the four-uma loops behind the rotation are drawn with four members, the "All four" tab and
  // the 12 careers
  async fourloop(b) {
    const pg = await open(b);
    await pg.selectOption('#c-top', '3'); await settle(pg);
    const sizes = await pg.evaluate(() => [...document.querySelectorAll('#results .loop')].map(l => l.querySelectorAll('.mem').length));
    check('three results: a rotation first, then four-uma loops', sizes.length === 3 && sizes[0] === 3 && sizes.slice(1).every(n => n === 4 || n === 0), JSON.stringify(sizes));
    const i = sizes.indexOf(4), loop = pg.locator('#results .loop').nth(i);
    check('a four-uma loop: four members in four columns', i > 0 && await loop.evaluate((l) => { const m = l.querySelector('.members'); return !m.classList.contains('n3') && getComputedStyle(m).gridTemplateColumns.split(' ').length === 4; }));
    await loop.locator('.lacts button', { hasText: 'Race schedule' }).click(); await dialog(pg, 'sched-dlg');
    check('its schedule: a tab per member plus "All four"', (await pg.locator('#sched-dlg .stab').count()) === 5 && (await pg.locator('#sched-dlg .stab').last().innerText()) === 'All four');
    await pg.locator('#sched-dlg .stab').last().click();
    check('All four: a column per member', (await pg.locator('#sched-dlg table.sched').first().locator('th.mh').count()) === 4);
    await pg.keyboard.press('Escape');
    await loop.locator('.lacts button', { hasText: 'Affinity' }).click(); await dialog(pg, 'why-dlg');
    const rows = await pg.evaluate(() => [...document.querySelectorAll('#why-dlg table.tbl')].map(t => t.tBodies[0].rows.length));
    check('its affinity: 6 pairs and 12 careers, no grandparent is the trainee', rows.join(',') === '6,12' && !(await pg.locator('#why-dlg .node.g', { hasText: 'her own earlier veteran' }).count()), rows.join(','));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async borrowed(b) {
    const pg = await open(b);
    for (const nm of ['Light Hello', 'Sasami Anshinzawa']) await pg.locator('#cards .cgroup').first().locator('.ct', { hasText: nm }).first().click();   // the SSR section
    await settle(pg);
    const rows = await pg.evaluate(() => [...document.querySelectorAll('.deck .dcards')].map(r => [...r.querySelectorAll('.dc')]).filter(r => r.some(x => x.classList.contains('bor'))).map(r => r.findIndex(x => x.classList.contains('bor')) === r.length - 1));
    check('a deck borrows a card when the pals are not owned', rows.length > 0);
    check('the borrowed card is last in its row', rows.every(Boolean));
    check('the borrowed card is named as borrowed', await pg.evaluate(() => [...document.querySelectorAll('.dc.bor')].every(d => /borrowed/.test(d.getAttribute('aria-label')))));
    check('the borrowed card has the friend frame', await pg.evaluate(() => { const d = document.querySelector('.dc.bor'); return !!d && getComputedStyle(d, '::after').borderImageSource.startsWith('url'); }));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async popups(b) {
    const pg = await open(b);
    await pg.locator('.loop .lacts button', { hasText: 'Race schedule' }).first().click(); await dialog(pg, 'sched-dlg');
    check('schedule: a tab per member of the rotation plus "All three"', (await pg.locator('#sched-dlg .stab').count()) === 4 && (await pg.locator('#sched-dlg .stab').last().innerText()) === 'All three');
    const grids = await pg.evaluate(() => [...document.querySelectorAll('.ag-grid')].map(g => { const r = g.getBoundingClientRect(); return Math.round(r.width) + 'x' + Math.round(r.height); }));
    check('agenda: three identical year grids', grids.length === 3 && new Set(grids).size === 1, grids.join(' '));
    check('agenda: a note fills the blank Junior half-year', /starts in Junior Year/.test(await pg.locator('#sched-dlg .ag-pre').innerText()));
    check('agenda: fits the window without scrolling', await pg.evaluate(() => { const d = document.querySelector('#sched-dlg .dlg'); return d.scrollHeight <= d.clientHeight + 1; }));
    check('agenda: every race has its banner', (await pg.locator('#sched-dlg .ag-race').count()) === (await pg.locator('#sched-dlg .ag-race img.banner').count()));
    check('agenda: a race\'s "x% win" sits in its turn bar, its tooltip says why', await pg.evaluate(() => [...document.querySelectorAll('#sched-dlg .ag-slot.on .ag-when')].every(w => /^(Early|Late) [A-Z][a-z]{2}\d+% win$/.test(w.textContent) && /^\d+% win: 110/.test(w.querySelector('b').title))));
    await pg.locator('#sched-dlg .stab').last().click(); check('All three shows the comparison table, a column per member', (await pg.locator('#sched-dlg table.sched').count()) > 0
      && (await pg.locator('#sched-dlg table.sched').first().locator('th.mh').count()) === 3);
    check('All three: turns written as on the member tabs ("Early Apr"), no shared/optional key', await pg.evaluate(() => [...document.querySelectorAll('#sched-dlg td.when')].every(t => /^(Early|Late) [A-Z][a-z]{2}$/.test(t.textContent)))
      && await pg.locator('#sched-dlg .agkey').isHidden());
    await pg.keyboard.press('Escape');
    await pg.locator('.loop .lacts button', { hasText: 'Affinity' }).first().click(); await dialog(pg, 'why-dlg');
    const rows = await pg.evaluate(() => [...document.querySelectorAll('#why-dlg table.tbl')].map(t => t.tBodies[0].rows.length));
    check('affinity: a rotation has 3 pairs and 3 careers', rows.join(',') === '3,3', rows.join(','));
    // the trainee is her own grandparent under each parent: labelled, no relation points
    const self = await pg.evaluate(() => [...document.querySelectorAll('#why-dlg .node.g')].filter(n => /her own earlier veteran/.test(n.innerText)).map(n => /(^|\s)0 relations/.test(n.innerText)));
    check('affinity: in a rotation the trainee is her own grandparent twice, with 0 relation points; the rotation is explained',
      self.length === 2 && self.every(Boolean) && /three-uma rotation/.test(await pg.locator('#why-dlg .afx').first().innerText()), JSON.stringify(self));
    const af = await pg.evaluate(() => ({ faces: document.querySelectorAll('#why-dlg .aface').length, fx: [...document.querySelectorAll('#why-dlg .fx')].map(e => e.textContent).join(' | '), hidden: getComputedStyle(document.querySelector('#why-dlg .aface .an')).display }));
    check('affinity: umas as portraits (names hidden until asked; four grandparents per career), the formula with P, P2, GP1, GP2', af.faces >= 3 * 2 + 3 * 6 && af.hidden === 'none' && /GP1/.test(af.fx) && /P1–P2/.test(af.fx), JSON.stringify(af));
    const ib = pg.locator('#why-dlg .oinfo'), mathVis = () => pg.locator('#why-math').isVisible();
    const m0 = await mathVis(); await ib.click(); const m1 = await mathVis(); await ib.click();
    check('affinity: the formula sits behind the (i) by the title (closed, opens, closes)', !m0 && m1 && !(await mathVis()) && (await ib.getAttribute('aria-expanded')) === 'false', JSON.stringify([m0, m1]));
    const pos = () => pg.evaluate(() => [...document.querySelectorAll('#why-dlg table.tbl td')].slice(0, 40).map(td => Math.round(td.getBoundingClientRect().left) + ',' + Math.round(td.getBoundingClientRect().top)).join(' '));
    const before = await pos(); await pg.locator('#why-dlg .aface').nth(3).hover();
    check('affinity: hovering a portrait shows the name without moving the table', (await pos()) === before && await pg.locator('#why-dlg .aface').nth(3).locator('.an').isVisible());
    await pg.locator('#why-dlg .aface').first().click();
    check('affinity: tapping a portrait shows her name', await pg.locator('#why-dlg .aface.open .an').first().isVisible());
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
    for (const f of [0, 0.5, 1]) { await pg.evaluate((y) => scrollTo(0, y), f * H);   // the button follows an IntersectionObserver
      seen.push(await until(pg, () => { const p = document.getElementById('run-pill'), r = p.getBoundingClientRect(), c = document.getElementById('rcol').getBoundingClientRect();
        const resIn = c.bottom > 0 && c.top < innerHeight, shown = getComputedStyle(p).display !== 'none'; return shown === !resIn && (!shown || (r.top >= 0 && r.bottom <= innerHeight)); })); }
    check('phone: the status button shows (on screen) exactly when the results are out of view', seen.every(Boolean), seen.join(','));
    check('phone: it shows the headline odds', /^1 in 107 · see result/.test(await pg.locator('#run-pill').innerText()), await pg.locator('#run-pill').innerText());
    // edit a setting far down, then the button takes you back to the result
    const ct = pg.locator('#cards .ct').nth(2); await ct.scrollIntoViewIfNeeded(); await ct.click(); await settle(pg); await ct.click(); await settle(pg);
    await pg.click('#run-pill');   // a smooth scroll
    check('phone: after editing a card, the status button scrolls back to the results', await until(pg, () => { const r = document.getElementById('rcol').getBoundingClientRect(); return Math.abs(r.top) < 40 && document.activeElement.id === 'rcol'; }));
    // the roster and card boxes: shorter than the screen, their bottom edge fading while there is more below
    const boxes = await pg.evaluate(() => ['roster', 'cards'].map(id => { const e = document.getElementById(id); return { h: e.clientHeight / innerHeight, more: e.classList.contains('more') }; }));
    const part = await pg.evaluate(() => { const s = new Set(); let over = 0; for (const t of document.querySelectorAll('#cards .ct')) { const c = t.querySelector('.cav').getBoundingClientRect();
      s.add(Math.round(c.width)); if (c.width > t.getBoundingClientRect().width + 0.5) over++; } return { sizes: [...s], over }; });
    check('phone: every card picture is the same size and inside its tile, loaded or not', part.sizes.length === 1 && !part.over, JSON.stringify(part));
    check('phone: the roster and card boxes are at most 55% of the screen high and show that they scroll', boxes.every(x => x.h <= 0.56 && x.more), JSON.stringify(boxes));
    await pg.evaluate(() => scrollTo(0, 0));
    await pg.click('#x-scen');
    check('phone: the open scenario picker fits the screen', await pg.evaluate(() => { const r = document.getElementById('scen-menu').getBoundingClientRect();
      return r.width > 0 && r.left >= 0 && r.right <= innerWidth && document.documentElement.scrollWidth <= document.documentElement.clientWidth; }));
    const ph = await pg.evaluate(() => [...document.querySelectorAll('#scen-menu .stile')].map(t => { const r = t.getBoundingClientRect(), s = t.querySelector('.ssub').getBoundingClientRect();
      return Math.round(r.width) + 'x' + Math.round(r.height) + ' line ' + Math.round(s.height); }));
    check('phone: the scenario tiles are the same size, each logo + one line', new Set(ph.map(x => x.split(' ')[0])).size === 1 && ph.every(x => +x.split(' ').at(-1) <= 22), ph.join(' | '));
    await pg.keyboard.press('Escape');
    await pg.click('#rank-q');
    check('phone: the open lineage-rank popup fits the screen', await pg.evaluate(() => { const r = document.getElementById('rank-note').getBoundingClientRect();
      return r.width > 0 && r.left >= 0 && r.right <= innerWidth && document.documentElement.scrollWidth <= document.documentElement.clientWidth; }));
    await pg.keyboard.press('Escape');
    await pg.locator('.loop .lacts button', { hasText: 'Race schedule' }).first().click(); await dialog(pg, 'sched-dlg');
    check('phone: agenda stacks without the blank Junior half-year', (await pg.locator('#sched-dlg .ag-pre:visible').count()) === 0);
    await pg.evaluate(() => { const d = document.querySelector('#sched-dlg .dlg'); d.scrollTo(0, d.scrollHeight); });
    const closeVis = () => pg.evaluate(() => { const b = [...document.querySelectorAll('dialog[open] .dhead button')].find(x => x.textContent === 'Close'), r = b.getBoundingClientRect(); return r.top >= 0 && r.bottom <= innerHeight; });
    check('phone: the schedule\'s Close stays on screen at the bottom of the dialog', await closeVis());
    await pg.locator('#sched-dlg .stab').last().click();
    check('phone: the All three table fits the 390 px screen', await pg.evaluate(() => [...document.querySelectorAll('#sched-dlg .tablewrap')].every(w => w.scrollWidth <= w.clientWidth)));
    await pg.keyboard.press('Escape');
    await pg.locator('.loop .lacts button', { hasText: 'Affinity' }).first().click(); await dialog(pg, 'why-dlg');
    await pg.evaluate(() => { const d = document.querySelector('#why-dlg .dlg'); d.scrollTo(0, d.scrollHeight); });
    check('phone: the affinity\'s Close stays on screen too', await closeVis());
    await pg.keyboard.press('Escape');
    await pg.click('#ex-open'); check('the export dialog\'s Close sits in the same header row as the others', await pg.evaluate(() => document.getElementById('ex-close').closest('.dhead') !== null)); await pg.keyboard.press('Escape');
    await pg.setViewportSize({ width: 1280, height: 800 });   // the page reorders on its resize handler
    check('widened to desktop, the results go back beside (after) the settings', await until(pg, () => !!(document.querySelector('.rail').compareDocumentPosition(document.getElementById('rcol')) & Node.DOCUMENT_POSITION_FOLLOWING)));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async nonecards(b) {
    // "None owned" on support cards: no owned card reaches the solver (an empty owned list used to mean "all owned")
    const pg = await open(b); const all = await odds(pg);
    await pg.click('#c-none'); await pg.click('#c-borrow'); await settle(pg);
    check('None owned: the count says "0 of N listed cards owned"', /^0 of \d+ listed cards owned$/.test(await pg.locator('#c-count').innerText()), await pg.locator('#c-count').innerText());
    const job = await pg.evaluate(() => window.__runs.at(-1));
    check('None owned + no borrowing: the job allows no card and the odds drop', job.owned_cards.length === 1 && job.owned_cards[0] === 0 && !job.borrow && (await odds(pg)) !== all, (await odds(pg)) + ' vs ' + all);
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async reload(b) {
    // a reload with non-default settings shows their last result from the first frame (not the default example), and
    // the fresh identical result doesn't redraw it
    const pg = await open(b), ctx = pg.context(), def = await odds(pg);
    await pg.click('#rank button[data-v="0.20,0.70,0.10"]'); await settle(pg);   // rank SS: a different result, quick to search
    const mine = await odds(pg);
    await ctx.addInitScript(() => { window.__seen = []; const snap = () => { const o = document.querySelector('#results .loop .odds'); window.__seen.push(o ? o.textContent : '');
      if (o && !o.__mark) o.__mark = 1; if (performance.now() < 6000) requestAnimationFrame(snap); }; requestAnimationFrame(snap); });
    await pg.reload(); await settle(pg);
    const seen = [...new Set(await pg.evaluate(() => window.__seen))];
    check('reload: the saved settings\' result from the first frame, never the default example', seen.length === 1 && seen[0] === mine && mine !== '' && mine !== def, seen.join(' | ') + ' · default ' + def);
    check('reload: an identical fresh result is not redrawn', await pg.evaluate(() => document.querySelector('#results .loop .odds').__mark === 1));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await ctx.close();
  },
  async delivery(b) {
    // self-hosted fonts (no third-party requests), a real favicon, the solver's files fingerprinted, the first roster
    // portraits eager, and accessible names that start with the visible text (voice control)
    const hosts = new Set(); const urls = [];
    const pg = await open(b, { before: (p) => p.on('request', r => { const u = new globalThis.URL(r.url()); if (u.protocol.startsWith('http')) { hosts.add(u.host); urls.push(u.pathname); } }) });
    check('no third-party requests (fonts self-hosted)', hosts.size === 1, [...hosts].join(' '));
    check('the solver data and binary come from fingerprinted files', urls.some(u => /^\/assets\/engine-global\.[0-9a-f]{10}\.json$/.test(u)) && urls.some(u => /^\/assets\/loopopt\.[0-9a-f]{10}\.wasm$/.test(u)), urls.filter(u => u.startsWith('/assets/') && !u.includes('/img.')).join(' '));
    check('page fonts are the self-hosted subsets', await pg.evaluate(async () => { await document.fonts.ready; return [...document.fonts].some(f => f.family.includes('Zen Kaku') && f.status === 'loaded'); }));
    const ico = await pg.request.get(new globalThis.URL('favicon.ico', URL).href);
    check('favicon.ico is an icon, not the page', ico.ok() && /image\//.test(ico.headers()['content-type'] || ''), ico.headers()['content-type']);
    check('the first roster portraits load eagerly', await pg.evaluate(() => { const im = [...document.querySelectorAll('#roster .av img')]; return im.length > 8 && im.slice(0, 8).every(i => i.loading === 'eager') && im.slice(8).every(i => i.loading === 'lazy'); }));
    check('sort buttons are named by their visible text first', await pg.evaluate(() => ['rs', 'cs'].every(id => { const e = document.getElementById(id); return e.getAttribute('aria-label').startsWith(e.textContent.trim()); })));
    check('card tiles take their name from their visible text', await pg.evaluate(() => [...document.querySelectorAll('#cards .ct')].every(e => !e.hasAttribute('aria-label') && e.hasAttribute('aria-pressed'))));
    await pg.context().close();
  },
  async gm(b) {
    // the era buttons: switching to the Grand Masters-era data (fetched, the status line says so meanwhile) opens on
    // Grand Masters (rank SS, no Unity golds) and searches with it; back on Global the scenario is Unity Cup again
    const pg = await open(b);
    await pg.route('**/assets/data-gm.*', async (r) => { await new Promise(x => setTimeout(x, 1500)); await r.continue(); });   // a slow fetch of the GM era's data
    await navClick(pg, '#era-gm');
    check('GM era: while its data loads, the status line says so', await until(pg, () => /Loading the JP Grand Masters-era data/.test(document.getElementById('run-note').textContent)), await note(pg));
    await settle(pg); await pg.unroute('**/assets/data-gm.*');
    check('switching to the GM era selects Grand Masters, at rank SS, and searches with it', (await scen(pg)) === 'grandmasters' && (await pg.getAttribute('#x-scen', 'aria-label')) === 'Scenario: Grand Masters'
      && (await pg.getAttribute('#rank', 'data-value')) === '0.20,0.70,0.10' && /Searched/.test(await note(pg)) && (await pg.locator('#results .loop').count()) > 0
      && await pg.evaluate(() => window.__runs.at(-1).scenario === 'grandmasters' && window.__runs.at(-1).star_dist.join() === '0.2,0.7,0.1'), await odds(pg));
    check('Grand Masters gives no guaranteed golds', (await pg.getAttribute('#g-20049', 'data-g')) === '0' && (await pg.getAttribute('#g-20046', 'data-g')) === '0');
    const umas = (await pg.evaluate(() => fetch(JSON.parse(document.getElementById('data-assets').textContent).dataGm).then(r => r.json()))).umas; const nOut = umas.reduce((a, u) => a + Math.max(1, u.outfits.length), 0);
    check('GM era: roster from the GM data', (await pg.locator('#roster .ut').count()) === nOut, nOut + ' outfits');
    // the GM era offers Grand Masters as a fifth tile (same size as the others)
    await pg.click('#x-scen');
    const gt = await pg.evaluate(() => [...document.querySelectorAll('#scen-menu .stile')].map(t => { const r = t.getBoundingClientRect(); return { v: t.dataset.v, sub: t.querySelector('.ssub').innerText.replace(/\s+/g, ' ').trim(), size: Math.round(r.width) + 'x' + Math.round(r.height), x: Math.round(r.left + r.width / 2) }; }));
    const mid = await pg.evaluate(() => { const r = document.getElementById('scen-menu').getBoundingClientRect(); return Math.round(r.left + r.width / 2); });
    check('GM era: five equal tiles, Grand Masters last and centred', gt.length === 5 && gt[4].v === 'grandmasters' && gt[4].sub === 'Hint rate ×1.44 (estimate)' && new Set(gt.map(t => t.size)).size === 1 && Math.abs(gt[4].x - mid) <= 1, JSON.stringify(gt));
    if (process.env.LLP_SHOTS) await pg.locator('#scen-menu').screenshot({ path: `${process.env.LLP_SHOTS}/${b.browserType().name()}-scen-picker-gm.png` });
    await pg.keyboard.press('Escape');
    await navClick(pg, '#era-global'); await settle(pg);
    check('back on Global: Unity Cup with its golds, rank below SS', (await scen(pg)) === 'unity' && (await pg.getAttribute('#rank', 'data-value')) === '0.50,0.45,0.05'
      && (await pg.getAttribute('#g-20049', 'data-g')) === '2' && await pg.evaluate(() => window.__runs.at(-1).scenario === 'unity'));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async settings(b) {
    // saved settings load: roster and cards per data set, the rest shared
    const pg = await open(b, { init: () => { try { if (sessionStorage.getItem('seeded')) return; sessionStorage.setItem('seeded', '1'); localStorage.clear();
      localStorage.setItem('llp:settings', JSON.stringify({ v: 2, era: 'global', common: { rank: '0.20,0.70,0.10' }, global: { roster: { 1001: 'out' }, cardsOff: { 30052: false } }, gm: { roster: { 1002: 'out' } } })); } catch {} } });
    check('saved settings load: the uma marked not owned shows not owned', await pg.evaluate(() => [...document.querySelectorAll('#roster .ut[data-uma="1001"]')].every(t => t.dataset.s === 'out')));
    check('… card ownership and rank too', await pg.evaluate(() => document.querySelectorAll('#cards .ct[aria-pressed="false"]').length === 1) && (await pg.getAttribute('#rank', 'data-value')) === '0.20,0.70,0.10');
    // changes persist across a reload
    await pg.locator('#roster .ut .up').nth(3).click(); await settle(pg); await pg.reload(); await settle(pg);
    check('a change survives a reload', (await pg.locator('#roster .ut[data-s="out"]').count()) >= 1 + (await pg.locator('#roster .ut[data-uma="1001"]').count()));
    // export, then import the same text
    await pg.click('#ex-open'); const text = await pg.inputValue('#ex-text');
    await pg.fill('#ex-text', text); await pg.click('#ex-load');
    check('export then import: settings load', (await pg.locator('#ex-msg').innerText()) === 'Settings loaded.' && JSON.parse(text).settings.era === 'global');
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
    const p3 = await open(b, { init: () => { try { if (sessionStorage.getItem('seeded')) return; sessionStorage.setItem('seeded', '1'); localStorage.clear();
      localStorage.setItem('llp:settings', JSON.stringify({ v: 2, era: 'global', common: { scen: 'grandmasters', rank: '0.20,0.70,0.10', targets: [{ group: 20049 }, { group: 20046 }, { group: 20159 }] }, global: {}, gm: {} })); } catch {} } });
    check('Grand Masters saved, Global data active: Unity Cup is used (its golds and rank)', (await p3.getAttribute('#x-scen', 'data-value')) === 'unity'
      && await p3.evaluate(() => window.__runs.at(-1).scenario === 'unity') && (await p3.getAttribute('#rank', 'data-value')) === '0.50,0.45,0.05' && (await p3.getAttribute('#g-20049', 'data-g')) === '2');
    await p3.context().close();
  },
  async about(b) {
    // "How this works": a link in the header to its own page (/about), which links back
    const pg = await open(b);
    await pg.locator('header.top .tnav a', { hasText: 'How this works' }).click(); await pg.waitForLoadState();
    check('header link opens /about', new globalThis.URL(pg.url()).pathname === '/about' && /How this works/i.test(await pg.locator('main.about h2').innerText()));
    const spec = pg.locator('main.about details.spec');
    check('/about: the detailed model starts folded and opens to the model text; the version shows', !(await spec.getAttribute('open') !== null) && !/Model\./.test(await pg.locator('main.about').innerText())
      && await (async () => { await spec.locator('summary').click(); return /Model\./.test(await spec.innerText()); })() && /Version v?\S+/.test(await pg.locator('main.about').innerText()));
    if (b.browserType().name() === 'chromium') { const f = await pg.evaluate(AUDIT); check('/about: every text run meets WCAG AAA', !f.length, f.slice(0, 5).join(' | ')); }
    await pg.locator('header.top .tnav a').click(); await pg.waitForSelector('#targets');
    check('/about links back to the planner', new globalThis.URL(pg.url()).pathname === '/');
    await pg.context().close();
  },
  async dark(b) {
    // dark mode: the system's preference until the header button is used, then the visitor's choice, kept in this browser
    const pg = await open(b, { scheme: 'dark' });
    const theme = () => pg.evaluate(() => document.documentElement.getAttribute('data-theme'));
    check('dark: with no saved choice the page follows a dark system preference', (await theme()) === 'dark'
      && await pg.evaluate(() => getComputedStyle(document.body).backgroundColor !== 'rgb(243, 245, 248)' && document.getElementById('theme-btn').getAttribute('aria-pressed') === 'true'));
    await pg.emulateMedia({ colorScheme: 'light' });
    await pg.waitForFunction(() => document.documentElement.getAttribute('data-theme') === 'light', null, { timeout: 3000 }).catch(() => {});   // the change event arrives asynchronously
    check('dark: … and follows the system when it changes', (await theme()) === 'light');
    await pg.emulateMedia({ colorScheme: 'dark' });
    await pg.waitForFunction(() => document.documentElement.getAttribute('data-theme') === 'dark', null, { timeout: 3000 }).catch(() => {});
    if (b.browserType().name() === 'chromium') {
      await pg.waitForTimeout(300);   // the results' fade back from the searching dim
      const fails = [...await pg.evaluate(AUDIT)];
      await pg.click('#x-scen'); fails.push(...await pg.evaluate(AUDIT)); await pg.keyboard.press('Escape');
      await pg.click('#rank-q'); fails.push(...(await pg.evaluate(AUDIT)).filter(x => /rtab|rfoot|gpop/.test(x))); await pg.keyboard.press('Escape');
      await pg.locator('.deck .dc').first().click(); fails.push(...(await pg.evaluate(AUDIT)).filter(x => /cpop|sw|gtl|ctt/.test(x))); await pg.keyboard.press('Escape');
      await pg.locator('.loop .lacts button', { hasText: 'Race schedule' }).first().click(); await dialog(pg, 'sched-dlg'); fails.push(...await pg.evaluate(AUDIT));
      await pg.locator('#sched-dlg .stab').last().click(); fails.push(...await pg.evaluate(AUDIT)); await pg.keyboard.press('Escape');
      await pg.locator('.loop .lacts button', { hasText: 'Affinity' }).first().click(); await dialog(pg, 'why-dlg'); fails.push(...await pg.evaluate(AUDIT)); await pg.keyboard.press('Escape');
      check('dark: every text run meets WCAG AAA (page, scenario picker, rank popup, card popup, schedule, affinity)', !fails.length, fails.slice(0, 5).join(' | '));
    }
    await pg.click('#theme-btn');
    check('dark: the header button switches to light and remembers it', (await theme()) === 'light' && await pg.evaluate(() => localStorage.getItem('llp:theme') === 'light'));
    await pg.reload(); await settle(pg);
    check('dark: … the choice outlives a reload and beats the system preference', (await theme()) === 'light');
    await pg.emulateMedia({ colorScheme: 'light' }); await pg.emulateMedia({ colorScheme: 'dark' }); await pg.waitForTimeout(300);
    check('dark: … and a system change no longer overrides it', (await theme()) === 'light');
    await pg.click('#theme-btn');
    check('dark: the button switches back to dark', (await theme()) === 'dark');
    await pg.goto(URL.replace(/\/?$/, '/') + 'about'); await pg.waitForLoadState('load');
    check('dark: /about takes the same choice', (await theme()) === 'dark');
    if (b.browserType().name() === 'chromium') { const f = await pg.evaluate(AUDIT); check('dark: /about meets WCAG AAA', !f.length, f.slice(0, 5).join(' | ')); }
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async career(b) {
    // the career planner (/career): the lineage tree, the parent picker, the skills prompt, the settings dialogs,
    // umadump import, the veteran editor, the search and its results
    const ctx = await b.newContext({ viewport: { width: 1440, height: 1000 } });
    await ctx.addInitScript(() => {
      const W = window.Worker; window.__careerJobs = []; window.__careerResults = [];
      window.Worker = function (...args) { const w = new W(...args), post = w.postMessage.bind(w);
        w.addEventListener('message', (e) => { if (e.data?.type === 'result') window.__careerResults.push(e.data.r); });
        w.postMessage = (m, ...rest) => { if (m?.type === 'run') window.__careerJobs.push(m.job); return post(m, ...rest); }; return w; };
    });
    const pg = await ctx.newPage(); pg.errors = [];
    pg.on('pageerror', e => pg.errors.push(String(e))); pg.on('console', m => m.type() === 'error' && !/downloadable font/.test(m.text()) && pg.errors.push(m.text()));
    const note = () => pg.textContent('#run-note');
    const done = async () => { await pg.waitForTimeout(700);   // past the pause before a search
      await pg.waitForFunction(() => { const n = document.getElementById('run-note'); return !n.querySelector('.spin') && n.textContent.length > 0; }, null, { timeout: 180000 }); };
    const ask = async (label) => { await dialog(pg, 'ask-dlg'); await pg.locator('#ask-btns button', { hasText: label }).click(); };
    await pg.goto(URL.replace(/\/?$/, '/') + 'career'); await done();
    check('career: with no veterans it asks for parents or an import', /Choose both parents, or import/.test(await note()), await note());
    check('career: the toolbar shows the shared settings', (await pg.textContent('#o-skills-n')) === '8' && await pg.locator('.rbbar .tbtn .tico').count() === 3);
    await pg.click('#o-skills'); await dialog(pg, 'set-skills');
    check('career: Target skills opens the planner\'s skill editor', await pg.locator('#set-skills #targets .tgt').count() === 8);
    await pg.locator('#set-skills .dclose').click();
    await pg.locator('#v-file').setInputFiles(new globalThis.URL('../val/umadump/U_trained_chara_data.json', import.meta.url).pathname);
    await pg.waitForFunction(() => document.getElementById('v-msg').textContent);   // the file is read asynchronously
    check('career: the umadump file imports 12 veterans', /Imported 12 veterans/.test(await pg.textContent('#v-msg')) && await pg.locator('#vets .vet').count() === 12);
    await done();
    // Export: a dialog with the veterans and the solver job as text; Copy puts it on the clipboard, Download saves it, Paste takes it back
    await pg.click('#v-export'); await dialog(pg, 'vx-dlg');
    const exText = await pg.inputValue('#vx-text'), ex = JSON.parse(exText);
    const chromium = b.browserType().name() === 'chromium';
    if (chromium) await ctx.grantPermissions(['clipboard-read', 'clipboard-write']);
    await pg.click('#vx-copy'); await pg.waitForFunction(() => document.getElementById('vx-msg').textContent);
    const copied = (await pg.textContent('#vx-msg')) === 'Copied.' && (!chromium || await pg.evaluate(() => navigator.clipboard.readText()) === exText);
    const [dl] = await Promise.all([pg.waitForEvent('download'), pg.click('#vx-dl')]);
    const dlSame = (await import('fs')).readFileSync(await dl.path(), 'utf8') === exText;
    await pg.locator('#vx-dlg .dclose').click();
    await pg.click('#v-paste'); await dialog(pg, 'paste-dlg'); await pg.fill('#paste-text', exText); await pg.click('#paste-load');
    check('career: Export shows the veterans and the search job as text, Copy copies it, Download saves it, and Paste takes it back', ex.app === 'uma-legacy-veterans'
      && ex.veterans.length === 12 && ex.job && ex.job.cmd === 'career' && ex.job.legacies.length === 12 && copied && dlSame
      && /Imported 12 veterans from an export/.test(await pg.textContent('#v-msg')) && await pg.locator('#vets .vet').count() === 12 && !(await pg.locator('#paste-dlg[open]').count()),
      `copy: ${await pg.textContent('#vx-msg')}, download same: ${dlSame}`);
    // the goal's steps: 8/8 veterans of two different umas -> step 3, the career uses both as parents; then restore
    const goalEx = JSON.parse(exText), all8 = Object.fromEntries(goalEx.job.targets.map(t => [t.group, 2]));
    for (const v of goalEx.veterans) for (const g of v.parents) g.sparks = {};   // (the test file's parents carry every spark: they would make a looped parent at once)
    const two = [0, goalEx.veterans.findIndex(v => v.chara !== goalEx.veterans[0].chara)]; for (const k of two) goalEx.veterans[k].sparks = { ...all8 };
    await pg.click('#v-paste'); await dialog(pg, 'paste-dlg'); await pg.fill('#paste-text', JSON.stringify(goalEx)); await pg.click('#paste-load'); await done();
    const goalJob = await pg.evaluate(() => window.__careerJobs.filter(j => j.cmd === 'career' && !j.next && !j.compare).at(-1));
    check('career: with 8/8 veterans of two umas the goal is at step 3 and the career uses both as parents', /Step 3 of 3/.test(await pg.textContent('#goal-note').catch(() => ''))
      && goalJob && goalJob.fix.length === 2 && goalJob.fix.includes(0) && goalJob.fix.includes(two[1]), JSON.stringify(goalJob && goalJob.fix));
    await pg.click('#v-paste'); await dialog(pg, 'paste-dlg'); await pg.fill('#paste-text', exText); await pg.click('#paste-load');
    await done(); await pg.waitForFunction(() => /Planned in/.test(document.getElementById('run-note').textContent), null, { timeout: 300000 });   // the restored pool's plan
    await done();
    check('career: the planner weighs every career, checked on exact odds', /Weighed 3,564 careers.*Checked on exact odds/.test(await note()), await note());
    check('career: the tree shows the planner\'s trainee and parents', await pg.locator('#tree .pn.ghost').count() === 3 && await pg.locator('#tree .pn.gp').count() === 4);
    check('career: the odds, a six-card deck and eight target rows under the tree, as the loop planner shows them', /1 in .* runs to land all 8/.test(await pg.textContent('#results .lhead'))
      && await pg.locator('#results > .deck .dc').count() === 6 && await pg.locator('.rtgt .cartab tbody tr').count() === 8);
    check('career: aptitude (pink) sparks show what they pass on', /Mile \d/.test(await pg.textContent('.pinkres')));
    check('career: the six legacies carry gate numbers 1-6 and their affinity, grandparents labelled plainly', (await pg.$$eval('#tree .pnh .gate', g => g.map(x => x.textContent).join(''))) === '123456'
      && await pg.locator('#tree .pn:not(.trainee) .aff').count() === 6 && (await pg.$$eval('#tree .pn.gp .pnl', l => l.every(x => x.textContent === 'Grandparent'))));
    const affs = await pg.$$eval('#tree .pn .aff', l => l.map(x => +x.textContent.replace(/\D/g, '')));
    const tsum = await pg.textContent('#tree .pn.trainee .tsum'), [pa, ga] = (tsum.match(/\d+/g) || []).map(Number);
    check('career: the trainee shows the affinity her six legacies hold, parents\' and grandparents\' parts', /Parents \d+ · grandparents \d+/.test(tsum)
      && Math.abs(affs[0] - (pa + ga)) <= 1 && Math.abs(affs.slice(1).reduce((a, x) => a + x, 0) - affs[0]) <= 6, tsum + ' ' + affs.join(','));
    // the planner's pick: the K best careers by odds, then the plan over them and a few more
    const lastJob = await pg.evaluate(() => window.__careerJobs.filter(j => j.cmd === 'career' && j.next && !j.compare).at(-1));
    const lastRes = await pg.evaluate(() => window.__careerResults.filter(r => r.next).at(-1));
    check('career: with places open the planner picks the loop\'s next career after the six best by odds', lastJob && lastJob.top === 6 && lastJob.per_trainee === false && !lastJob.rollout
      && lastRes && lastRes.next.options.length >= 2 && lastRes.next.loop_members.length === 3 && /^(loop|outside|best)$/.test(lastRes.next.rule) && /Planned in/.test(await note()));
    check('career: … the career on show is its pick, with why under the odds',
      (await pg.textContent('#tree .pn.trainee .nm')) === lastRes.next.choice.trainee.name && /^The planner's pick: /.test(await pg.textContent('#pick-why')), await pg.textContent('#pick-why'));
    // the deck: a card's panel, as in the planner; marking it not owned searches again without it
    const dc = pg.locator('#results > .deck .dc').first(), dcid = await dc.getAttribute('data-cid');
    await dc.click(); await pg.waitForSelector('.cpop:not([hidden]) [data-act="own"]');
    await pg.locator('.cpop [data-act="own"]').uncheck(); await done();
    check('career: a deck card\'s panel marks it not owned; the search drops it or borrows it', await pg.evaluate(id => JSON.parse(localStorage.getItem('llp:settings')).global.cardsOff[id] === false, dcid)
      && (!(await pg.locator(`#results > .deck .dc[data-cid="${dcid}"]`).count()) || await pg.locator(`#results > .deck .dc.bor[data-cid="${dcid}"]`).count() === 1));
    await pg.click('#o-cards'); await dialog(pg, 'set-cards'); await pg.click('#c-all'); await pg.locator('#set-cards .dclose').click(); await done();
    // the schedule: the planner's window, one trainee, from a button beside the deck
    check('career: the race schedule is a button in the deck\'s title row, not a section', await pg.locator('#results > .deck .dtitle #sched-open').count() === 1
      && !(await pg.locator('#results h3', { hasText: 'Race schedule' }).count()));
    await pg.click('#sched-open'); await dialog(pg, 'sched-dlg');
    const nRun = +(await pg.getAttribute('#sched-open', 'title')).match(/She runs (\d+) G1s/)[1];
    check('career: her schedule opens in the planner\'s schedule window: only the races she runs, one tab, the parent-won legend', nRun > 0
      && await pg.locator('#sched-dlg .ag-slot.on').count() === nRun && await pg.locator('#sched-dlg .ag-race').count() === nRun && !(await pg.locator('#sched-dlg .ag-race.wbtn').count())
      && /A parent won it/.test(await pg.textContent('#sched-dlg .agkey')) && await pg.locator('#sched-dlg .stab').count() === 1, `runs ${nRun}`);
    await pg.keyboard.press('Escape');
    // the options: per option this career's odds and its expected careers to land, the pick marked; no after-this-career section
    const opts = await pg.$$eval('.alts .optab tbody tr', ls => ls.map(l => [...l.children].map(c => c.textContent)));
    check('career: Options lists each option with "1 in X" for this career and its role, the pick marked and shown above', opts.length >= 2
      && opts.every(c => /^1 in [\d,.]+$/.test(c[1]) && /^(loop member|outside the loop|best odds now)$/.test(c[2])) && /^The loop: .+ → .+ → /.test(await pg.textContent('.alts .oloop')) && await pg.locator('.alts tr.on', { hasText: 'shown above' }).count() === 1
      && await pg.locator('.alts .otag.pick').count() === 1 && !(await pg.locator('#ahead-sec').count()), opts.map(c => c.join(' / ')).join(' | '));
    check('career: under the tree, the goal\'s step, the deck with its race schedule, the odds, the veteran analysis, then the target skills', await pg.evaluate(() => {
      const r = document.getElementById('results'), kids = [...r.children], at = (sel) => kids.findIndex(k => k.matches(sel)), sim = at('#sim-top');
      return document.getElementById('tree').nextElementSibling === r && at('#goal-note') === 0 && at('.deck') === 1 && !!kids[1].querySelector('.dtitle #sched-open') && at('.lhead') === 2
        && sim === 3 && at('.rtgt') === sim + 1 && kids[sim].querySelector('#simulation > h3')?.textContent === 'Veteran analysis'
        && !!kids[sim].querySelector('#simulation #sim-compare'); }));
    // her portrait beside her sparks and G1 wins, on a wide screen and a phone
    const simLayout = () => pg.evaluate(() => { const w = document.querySelector('#simulation .simwho'), f = w.querySelector(':scope > .av'), s = w.querySelector('.simskills');
      const fr = f.getBoundingClientRect(), sr = s.getBoundingClientRect();
      return f.getAttribute('role') === 'img' && /\S/.test(f.getAttribute('aria-label') || '') && Math.round(fr.width) === 96 && fr.right <= sr.left && fr.top < sr.bottom && sr.top < fr.bottom
        && w.contains(document.getElementById('sim-wins')) && s.getAttribute('aria-label') === 'Her sparks' && sr.right <= innerWidth
        && document.documentElement.scrollWidth <= document.documentElement.clientWidth; });
    const wide = await simLayout(); await pg.setViewportSize({ width: 390, height: 844 }); await pg.waitForTimeout(200);
    const phone = await simLayout(); await pg.setViewportSize({ width: 1440, height: 1000 }); await pg.waitForTimeout(200);
    check('career: the veteran analysis shows her portrait (named for screen readers) beside her sparks and G1 wins, also on a 390 px phone with no sideways scroll', wide && phone, `wide ${wide}, phone ${phone}`);
    const simStars = async (n) => {
      const loc = pg.locator('[data-k="sim:s20159"]');
      if (!(await loc.count())) { await pg.locator('#simulation .vskadd').click(); await pg.locator('#add-pop .addsk', { hasText: 'Uma Stan' }).click(); }
      for (let i = 0; i < 3; i++) { if (+(await loc.getAttribute('aria-label')).match(/(\d) star/)[1] === n) return; await loc.click(); }
    };
    const draft = () => pg.$$eval('#simulation .simskills .vskb', l => l.map(b => b.getAttribute('aria-label')).join('|'));
    const policyJobs = () => pg.evaluate(() => window.__careerJobs.filter(j => j.compare).length);
    const d0 = await draft();
    check('career: the veteran you got starts as the most likely result, and nothing is compared yet', d0.length > 0 && await policyJobs() === 0 && !(await pg.locator('#sim-verdict').count()));
    const original = await pg.evaluate(() => JSON.parse(localStorage.getItem('llp:career')).global.vets);
    await simStars(3);
    check('career: editing her leaves your veterans untouched and runs nothing', await pg.evaluate(old => JSON.stringify(JSON.parse(localStorage.getItem('llp:career')).global.vets) === JSON.stringify(old), original)
      && await policyJobs() === 0);
    await pg.click('#sim-reset');
    check('career: "Back to the most likely result" restores the same veteran', (await draft()) === d0);
    await simStars(3);
    await pg.click('#sim-compare');
    await pg.waitForSelector('#sim-verdict', { timeout: 180000 });
    check('career: Compare says, exactly, whether she improves your next run and shows the next career with her', /^She (improves|doesn't improve) your next run\./.test(await pg.textContent('#sim-verdict'))
      && /→/.test(await pg.textContent('#sim-result')) && !/Careers until one lands|Your best career today →/.test(await pg.textContent('#sim-result')));
    check('career: … the next career as portraits (trainee, then her two parents, her marked), its deck behind "Show its deck"', await pg.locator('#sim-result .nextc .cface').count() === 3
      && await pg.locator('#sim-result .nextc .cface .her').count() === 1 && await pg.locator('#sim-result details.nextdeck:not([open]) .deck .dc').count() === 6
      && /Show its deck/.test(await pg.textContent('#sim-result details.nextdeck summary')));
    check('career: the comparison sends her fixed stars, race indices and her parents', await pg.evaluate(() => {
      const j = window.__careerJobs.filter(j => j.compare).at(-1), v = j.legacies.at(-1);
      return j.compare === true && j.fix.length === 1 && j.fix[0] === j.legacies.length - 1 && v.sparks.some(([g, s]) => g === 20159 && s === 3)
        && v.wins.every(Number.isInteger) && v.parents.length === 2 && v.parents.every(p => p.chara > 0 && p.sparks.length > 0);
    }));
    for (const theme of ['light', 'dark']) {
      if (await pg.evaluate(() => document.documentElement.getAttribute('data-theme')) !== theme) await pg.click('#theme-btn');
      await pg.waitForTimeout(300);
      const fails = await pg.evaluate(AUDIT);
      check(`career: ${theme} mode with the outcome table and a comparison meets WCAG AAA`, !fails.length, fails.slice(0, 5).join(' | '));
    }
    await pg.click('#theme-btn');
    // a roll: a random result of this career, compared at once (stubbed random numbers: nothing lands, then everything at 3 stars)
    const roll = async (u) => { const before = await policyJobs(); await pg.evaluate(u => { const r = Math.random; Math.random = () => u; document.getElementById('sim-roll').click(); Math.random = r; }, u);
      await pg.waitForFunction(n => window.__careerJobs.filter(j => j.compare).length > n && document.querySelector('#sim-verdict'), before, { timeout: 180000 }); };
    await roll(0.999999999);
    const rolledAll = await pg.$$eval('#simulation .simskills .vsk:not(.pink) .vskb', l => l.map(b => b.getAttribute('aria-label')));
    check('career: "Roll a random result" draws from the career\'s odds and compares at once', rolledAll.length === 8 && rolledAll.every(x => /3 stars/.test(x))
      && await pg.evaluate(old => JSON.stringify(JSON.parse(localStorage.getItem('llp:career')).global.vets) === JSON.stringify(old), original));
    await roll(0);
    check('career: … a low roll lands nothing', await pg.locator('#simulation .simskills .vsk:not(.pink)').count() === 0);
    // rolls while a comparison runs: each cancels the one in flight (its worker goes), and only the last roll's runs and shows
    const policyResults = () => pg.evaluate(() => window.__careerResults.filter(r => r.compare).length);
    const [rj0, rr0] = [await policyJobs(), await policyResults()];
    const usable = await pg.evaluate(() => { const r = Math.random, ok = [];
      for (const u of [0.999999999, 0.999999999, 0]) { const b = document.getElementById('sim-roll'); ok.push(!b.disabled); Math.random = () => u; b.click(); Math.random = r; }
      return ok.every(Boolean) && !!document.querySelector('#sim-result .spin'); });
    await pg.waitForFunction(() => document.querySelector('#sim-verdict') && !document.querySelector('#sim-result .spin'), null, { timeout: 180000 });
    await pg.waitForTimeout(1500);   // time for a stray comparison to answer, were one still running
    const [rj, rr] = [await policyJobs() - rj0, await policyResults() - rr0];
    check('career: rolling again while a comparison runs cancels it: one comparison runs, for the last roll', usable && rj === 1 && rr === 1
      && await pg.locator('#simulation .simskills .vsk:not(.pink)').count() === 0 && await pg.evaluate(() => window.__careerJobs.filter(j => j.compare).at(-1).legacies.at(-1).sparks.length === 0),
      `usable ${usable}, jobs ${rj}, results ${rr}`);
    await pg.click('#sim-reset');
    check('career: … and the most likely result is back on reset', (await draft()) === d0);
    await simStars(3);
    await pg.click('#sim-compare'); await pg.waitForSelector('#sim-verdict', { timeout: 180000 });
    const nPolicy = await policyJobs();
    await simStars(1);
    check('career: an edit after Compare marks the comparison out of date without running another', /changed her since/.test(await pg.textContent('#sim-result')) && await policyJobs() === nPolicy);
    const baselineRuns = () => pg.evaluate(() => window.__careerJobs.filter(j => j.cmd === 'career' && !j.trainees.length && !j.fix.length && j.top === 1).length);
    const cachedRuns = await baselineRuns();
    await pg.getByRole('button', { name: 'Choose trainee' }).click(); await dialog(pg, 'pick-dlg');
    await pg.fill('#pick-filter', 'Agnes Digital'); await pg.locator('#pick-list .upick').first().click(); await pg.waitForTimeout(700);
    await pg.waitForSelector('#sim-compare:not([disabled])', { timeout: 180000 });
    // with the trainee chosen there is nothing for the plan to pick: Compare pressed twice restarts the comparison,
    // which finishes, and no plan job is sent
    const aheadJobs = () => pg.evaluate(() => window.__careerJobs.filter(j => j.next && !j.compare).length), la0 = await aheadJobs();
    await pg.evaluate(() => { document.getElementById('sim-compare').click(); document.getElementById('sim-compare').click(); });
    await pg.waitForFunction(() => document.querySelector('#sim-verdict') && !document.querySelector('#run-note .spin'), null, { timeout: 180000 });
    check('career: with the trainee chosen no plan runs, and Compare pressed twice finishes', await aheadJobs() === la0 && !/Planning|Planned in/.test(await note())
      && /^She (improves|doesn't improve) your next run\./.test(await pg.textContent('#sim-verdict')), await note());
    check('career: with the trainee fixed, Compare reuses today\'s best instead of searching it again', await baselineRuns() === cachedRuns);
    // her G1 wins: the G1 wins editor (each win in its slot; a click removes it, an empty slot's menu adds one); an edit
    // after Compare marks it out of date, nothing is saved to the pool, and the focus goes back to the wins button
    const winsN = async () => { const m = (await pg.textContent('#sim-wins')).match(/^(No|\d+) G1 win/); return m[1] === 'No' ? 0 : +m[1]; };
    const pj0 = await policyJobs(), w0 = await winsN();
    await pg.click('#sim-wins'); await dialog(pg, 'wins-dlg');
    const wb0 = pg.locator('#wins-dlg .ag-race.wbtn').first(), wname = (await wb0.getAttribute('aria-label')).replace(/, won: remove$/, '');
    const nShown = await pg.locator('#wins-dlg .ag-race.wbtn').count();
    await wb0.click();
    const freed = await pg.evaluate(() => document.activeElement.matches('#wins-dlg .ag-add') && document.activeElement.getAttribute('aria-label'));
    check('career: the entered veteran\'s G1 wins open on the calendar, one per slot; clicking one removes it, its slot offers to add one, the count follows', nShown === w0
      && await pg.locator('#wins-dlg .agenda .ag-year').count() === 3 && /G1 wins/.test(await pg.textContent('#wins-title')) && /: add a G1 win$/.test(freed || '')
      && await winsN() === w0 - 1 && (await pg.textContent('#wins-count')).startsWith(String(w0 - 1 || 'No')), `wins ${w0} -> ${await winsN()}, ${freed}`);
    await pg.keyboard.press('Enter');
    const picks = await pg.$$eval('#wins-dlg .wmenu:not([hidden]) .wpick', l => l.map(e => e.textContent));
    await pg.locator('#wins-dlg .wpick', { hasText: wname }).click();
    check('career: … the empty slot\'s menu lists the G1s held then (her schedule\'s first, with its odds); a pick puts the win back in that slot', picks[0]?.startsWith(wname) && /her schedule · \d+% win/.test(picks[0])
      && await winsN() === w0 && await pg.evaluate(n => document.activeElement.getAttribute('aria-label') === n + ', won: remove', wname), picks.join(' | '));
    check('career: … which marks the comparison out of date without running another, and saves nothing to your veterans', /changed her since/.test(await pg.textContent('#sim-result'))
      && await policyJobs() === pj0 && await pg.evaluate(old => JSON.stringify(JSON.parse(localStorage.getItem('llp:career')).global.vets) === JSON.stringify(old), original));
    await pg.keyboard.press('Escape');
    check('career: … closing it puts the focus back on the wins button', await pg.evaluate(() => document.activeElement?.id === 'sim-wins'));
    await pg.locator('#tree .pn.trainee').getByRole('button', { name: 'Let the planner pick' }).click(); await done();
    // editing a veteran keeps the result on screen while the search runs again
    const toteBefore = await pg.locator('#results > .lhead').count();
    await pg.locator('#vet-0 .vnode.self .vskb').first().click();
    check('career: editing a veteran keeps the result on screen until the new one is ready', toteBefore === 1 && await pg.locator('#results > .lhead').count() === 1 && await pg.locator('#tree .pn.ghost').count() === 3);
    await done();
    await pg.locator('#vet-0 .vnode.self .vskb').first().click(); await pg.locator('#vet-0 .vnode.self .vskb').first().click(); await done();
    // stars on a pool veteran: a click cycles 1 -> 2 -> 3 -> 1, and is saved
    const chip = pg.locator('#vet-0 .vnode.self .vskb').first();
    const stars = async () => +(await chip.getAttribute('aria-label')).match(/(\d) star/)[1];
    const s0 = await stars(); await chip.click(); const s1 = await stars(); await chip.click(); const s2 = await stars(); await chip.click(); const s3 = await stars();
    check('career: clicking a skill cycles its stars back round to 1', s1 === s0 % 3 + 1 && s2 === s1 % 3 + 1 && s3 === s2 % 3 + 1, [s0, s1, s2, s3].join(' '));
    check('career: … the focus stays on it, and the change is saved', await chip.evaluate(e => e === document.activeElement) && await pg.evaluate(() => /"sparks"/.test(localStorage.getItem('llp:career'))));
    const nsk = () => pg.locator('#vet-0 .vnode.self .vsk:not(.pink)').count(), n0 = await nsk();
    const first = await chip.getAttribute('aria-label');
    await pg.locator('#vet-0 .vnode.self .vsk').first().hover();
    const vis = await pg.locator('#vet-0 .vnode.self .vskx').first().evaluate(e => getComputedStyle(e).visibility);
    await pg.locator('#vet-0 .vnode.self .vskx').first().click();
    check('career: the x shows on hover and removes the skill', vis === 'visible' && await nsk() === n0 - 1);
    await pg.locator('#vet-0 .vnode.self .vskadd').click();
    const offered = await pg.$$eval('#add-pop .addsk', els => els.map(e => e.textContent));
    check('career: "+" offers the missing target, then aptitudes', offered.length === 1 && first.startsWith(offered[0]) && await pg.locator('#add-pop .addpk').count() > 0, offered.join(', '));
    await pg.locator('#add-pop .addsk').first().click();
    check('career: … which comes back at 2 stars', await nsk() === n0 && /2 stars/.test(await pg.locator('#vet-0 .vnode.self .vskb', { hasText: offered[0] }).getAttribute('aria-label')));
    await done();
    // a pool veteran's G1 wins: the same editor; Arima Kinen (Classic and Senior) is one race, moved from one turn to the other
    await pg.locator('#vet-0 .vnode.self .vwins').click(); await dialog(pg, 'wins-dlg');
    const savedWins = () => pg.evaluate(() => JSON.parse(localStorage.getItem('llp:career')).global.vets[0].wins);
    const arima = pg.locator('#wins-dlg .wbtn[data-ix="22"]'), had = (await savedWins()).includes(22), n0w = (await savedWins()).length;
    if (had) { await arima.click(); }
    const gone = !(await arima.count()) && (await savedWins()).length === n0w - (had ? 1 : 0);
    await pg.locator('#wins-dlg .ag-add[aria-label="Senior Year, Late Dec: add a G1 win"]').click();
    await pg.locator('#wins-dlg .wpick', { hasText: 'Arima Kinen' }).click();
    check('career: a veteran\'s G1 wins: a click removes Arima Kinen, the Senior Late Dec menu adds it back there, saved', gone && await arima.count() === 1
      && /Late Dec/.test(await arima.evaluate(e => e.closest('.ag-slot').textContent)) && await arima.evaluate(e => e.closest('.ag-year').classList.contains('y2'))
      && (await savedWins()).includes(22) && /\d+ G1 win/.test(await pg.textContent('#wins-count')), `had ${had}`);
    if (!had) await arima.click();
    await pg.locator('#wins-dlg .dclose').click(); await done();
    // parent 1: a new veteran, with the skills prompt
    await pg.locator('.pf').nth(0).getByRole('button', { name: 'Choose parent' }).click(); await dialog(pg, 'vpick-dlg');
    check('career: the parent picker lists your veterans', await pg.locator('#vp-list .vpt').count() === 12);
    await pg.click('#vp-tab-new'); await pg.fill('#vp-filter', 'Special Week'); await pg.locator('#vp-umas .upick').first().click();
    await dialog(pg, 'ask-dlg');
    check('career: picking a new uma asks whether to add the skills', /Add skills to Special Week/.test(await pg.textContent('#ask-title')));
    await ask('All 8 at 2★'); await done();
    const p1 = pg.locator('.pf').nth(0).locator('.pn.parent');
    check('career: … she is parent 1, with the eight targets at 2 stars', /Special Week/.test(await p1.textContent()) && !(await p1.getAttribute('class')).includes('ghost')
      && await p1.locator('.vskb').evaluateAll(els => els.length === 8 && els.every(e => /2 stars/.test(e.getAttribute('aria-label')))));
    check('career: … her parents are not known yet but carry the same eight sparks, and she joins your veterans', await pg.locator('.pf').nth(0).locator('.pn.gp', { hasText: 'Unknown' }).count() === 2
      && (await pg.locator('.pf').nth(0).locator('.pn.gp .vskb').evaluateAll(els => els.length === 16 && els.every(e => /2 stars/.test(e.getAttribute('aria-label')))))
      && await pg.locator('#vets .vet').count() === 13 && /Parent 1/.test(await pg.locator('#vet-12 .vtop').textContent()));
    check('career: the planner fills the rest', /Weighed 792 careers/.test(await note()), await note());
    // a grandparent: pick her uma, then her skills
    await pg.locator('.pf').nth(0).locator('.pn.gp .facebtn').first().click(); await dialog(pg, 'pick-dlg');
    await pg.fill('#pick-filter', 'Gold Ship'); await pg.locator('#pick-list .upick').first().click(); await ask('All 8 at 3★'); await done();
    const g1 = pg.locator('.pf').nth(0).locator('.pn.gp').first();
    check('career: a grandparent takes an uma and the skills chosen for her', /Gold Ship/.test(await g1.textContent())
      && await g1.locator('.vskb').evaluateAll(els => els.length === 8 && els.every(e => /3 stars/.test(e.getAttribute('aria-label')))));
    // the other parent can't be the same uma
    await pg.locator('.pf').nth(1).getByRole('button', { name: 'Choose parent' }).click(); await dialog(pg, 'vpick-dlg');
    const blocked = await pg.locator('#vp-list .vpt:disabled').count();
    await pg.locator('#vpick-dlg .dclose').click();
    check('career: veterans that can\'t be parent 2 are greyed out', blocked === 0 || await pg.locator('#vp-list .vpt:disabled .vptw').count() === blocked);
    // the trainee
    await pg.getByRole('button', { name: 'Choose trainee' }).click(); await dialog(pg, 'pick-dlg');
    await pg.fill('#pick-filter', 'Agnes Digital'); await pg.locator('#pick-list .upick').first().click(); await done();
    check('career: the chosen trainee is set and the picks list only parents', !(await pg.locator('#tree .pn.trainee').getAttribute('class')).includes('ghost')
      && (await pg.$$eval('.alts tbody .altw', ws => ws.every(w => /^Parents /.test(w.textContent.trim())))));
    await pg.locator('.alts tbody tr').filter({ has: pg.getByRole('button', { name: 'Use this' }) }).first().getByRole('button', { name: 'Use this' }).click(); await done();
    check('career: "Use this" sets the places; with all set the career is just scored', /Scored in/.test(await note()) && !(await pg.locator('.alts').count())
      && await pg.locator('#tree .pn.ghost').count() === 0, await note());
    await pg.locator('.pf').nth(1).getByRole('button', { name: 'Let the planner pick' }).click(); await done();
    check('career: "Let the planner pick" hands a place back', await pg.locator('#tree .pn.ghost').count() === 1 && /Weighed/.test(await note()));
    // a settings change searches again
    await pg.click('#o-umas'); await dialog(pg, 'set-umas');
    const umasNow = () => pg.evaluate(() => { const g = JSON.parse(localStorage.getItem('llp:settings')).global; return JSON.stringify([g.roster, g.outfits]); });
    const before = await umasNow(), searches = await pg.evaluate(() => window.__careerJobs.filter(j => j.cmd === 'career').length);
    await pg.locator('#set-umas #roster .ut .up').nth(3).click(); await pg.locator('#set-umas .dclose').click(); await done();
    check('career: the Umas dialog changes the owned umas and searches again', (await umasNow()) !== before
      && await pg.evaluate(n => window.__careerJobs.filter(j => j.cmd === 'career').length > n, searches), before);
    await pg.reload(); await done();
    check('career: the veterans and the chosen places outlive a reload', await pg.locator('#vets .vet').count() === 13
      && /Special Week/.test(await pg.locator('.pf').nth(0).textContent()) && /Agnes Digital/.test(await pg.locator('#tree .pn.trainee').textContent()));
    await pg.click('#v-paste'); await dialog(pg, 'paste-dlg'); await pg.fill('#paste-text', '{"x": 1}'); await pg.click('#paste-load');
    check('career: a paste that is not an export says so and changes nothing', /No veterans found/.test(await pg.textContent('#v-msg')) && await pg.locator('#vets .vet').count() === 13);
    await pg.keyboard.press('Escape');
    // Save one generation, then another: a veteran's parents become the next one's grandparents.
    const beforeSimulation = await pg.evaluate(() => JSON.parse(localStorage.getItem('llp:career')).global.vets.length);
    await simStars(3);
    await pg.click('#sim-add'); await done();
    check('career: adding her saves a labelled veteran and opens the next career', await pg.evaluate(n => {
      const c = JSON.parse(localStorage.getItem('llp:career')).global;
      const v = c.vets.at(-1);
      return c.vets.length === n + 1 && v.src === 'entered' && v.sparks[20159] === 3
        && v.parents.length === 2 && v.parents.every(p => p.chara > 0)
        && c.plan.trainee === null && c.plan.parents.every(p => p === null);
    }, beforeSimulation) && await pg.locator('#vets .simtag').count() === 1);
    const simulated = await pg.evaluate(() => JSON.parse(localStorage.getItem('llp:career')).global.vets.at(-1));
    await pg.reload(); await done();
    check('career: her ancestry survives reloading', await pg.evaluate(v => JSON.stringify(JSON.parse(localStorage.getItem('llp:career')).global.vets.at(-1)) === JSON.stringify(v), simulated));
    // Force this simulated veteran as a parent and complete another hypothetical career.
    await pg.evaluate(v => { const o = JSON.parse(localStorage.getItem('llp:career')); o.global.plan = { trainee: null, parents: [v.id, null] }; localStorage.setItem('llp:career', JSON.stringify(o)); }, simulated);
    await pg.reload(); await done();
    await pg.click('#sim-add'); await done();
    check('career: another generation keeps her as a grandparent', await pg.evaluate(v => {
      const o = JSON.parse(localStorage.getItem('llp:career')).global, next = o.vets.at(-1);
      return o.vets.length === v.count + 2 && next.parents.some(p => p.chara === v.chara && JSON.stringify(p.sparks) === JSON.stringify(v.sparks));
    }, { ...simulated, count: beforeSimulation }));

    await pg.locator('.pf').nth(0).getByRole('button', { name: 'Choose parent' }).click(); await dialog(pg, 'vpick-dlg');
    await pg.locator('#vp-list .vpt:not(:disabled)').last().click(); await done();
    const removal = await pg.evaluate(() => {
      const c = JSON.parse(localStorage.getItem('llp:career')).global;
      return { id: c.plan.parents.find(Boolean), count: c.vets.length };
    });
    const removalIndex = await pg.evaluate(id => JSON.parse(localStorage.getItem('llp:career')).global.vets.findIndex(v => v.id === id), removal.id);
    await pg.locator(`#vet-${removalIndex} .vdel`).click();
    check('career: removing a selected veteran immediately redraws and clears the fixed parent', await pg.locator('#vets .vet').count() === removal.count - 1
      && await pg.evaluate(({ id, count }) => { const c = JSON.parse(localStorage.getItem('llp:career')).global;
        return c.vets.length === count - 1 && !c.vets.some(v => v.id === id) && !c.plan.parents.includes(id); }, removal));
    await done();
    await pg.reload(); await done();
    check('career: removed veterans stay removed after reload', await pg.evaluate(id => !JSON.parse(localStorage.getItem('llp:career')).global.vets.some(v => v.id === id), removal.id));
    while (await pg.locator('#vets .vet').count() > 1) await pg.locator('#vets .vdel').last().click();
    await done();
    check('career: removing down to one veteran clears obsolete indexed results without a page error', await pg.locator('#vets .vet').count() === 1
      && /Choose both parents, or import/.test(await note()) && await pg.locator('#simulation').count() === 0);

    await pg.keyboard.press('Escape');
    if (b.browserType().name() === 'chromium') { await pg.waitForTimeout(300); const f = await pg.evaluate(AUDIT); check('career: every text run meets WCAG AAA', !f.length, f.slice(0, 5).join(' | ')); }
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await ctx.close();
  },
  async loopsize(b) {
    // the Loop setting: 3-uma rotations only, 4-uma loops only, or both (the default); saved, and sent to the solver
    const pg = await open(b);
    const members = () => pg.locator('.loop').first().locator('.mem').count();
    const def = await members();
    await pg.locator('#c-size [data-v="4"]').click(); await settle(pg);
    const four = await members(), job4 = await pg.evaluate(() => window.__runs.at(-1).sizes);
    await pg.locator('#c-size [data-v="3"]').click(); await settle(pg);
    const three = await members(), job3 = await pg.evaluate(() => window.__runs.at(-1).sizes);
    check('loop size: "4 umas" gives a 4-uma loop, "3 umas" a 3-uma rotation, and the job says so', four === 4 && three === 3 && JSON.stringify(job4) === '[4]' && JSON.stringify(job3) === '[3]', [def, four, three].join(' '));
    check('loop size: the choice is saved', await pg.evaluate(() => JSON.parse(localStorage.getItem('llp:settings')).common.loopSize === '3'));
    await pg.locator('#c-size [data-v="any"]').click(); await settle(pg);
    check('loop size: "3 or 4 umas" sends no restriction and returns the default result', await pg.evaluate(() => window.__runs.at(-1).sizes === undefined) && await members() === def);
    check('loop size: one radio checked, one in the Tab order', await pg.evaluate(() => { const bs = [...document.querySelectorAll('#c-size button')];
      return bs.filter(b => b.getAttribute('aria-checked') === 'true').length === 1 && bs.filter(b => b.tabIndex === 0).length === 1; }));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
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
    await pg.locator('.loop .lacts button', { hasText: 'Race schedule' }).first().click(); await dialog(pg, 'sched-dlg'); fails.push(...await pg.evaluate(AUDIT));
    await pg.locator('#sched-dlg .stab').last().click(); fails.push(...await pg.evaluate(AUDIT)); await pg.keyboard.press('Escape');
    await pg.locator('.loop .lacts button', { hasText: 'Affinity' }).first().click(); await dialog(pg, 'why-dlg'); fails.push(...await pg.evaluate(AUDIT));
    check('contrast: every text run meets WCAG AAA', !fails.length, fails.slice(0, 5).join(' | '));
    await pg.context().close();
    const p2 = await open(b, { before: (p) => p.route('**/img.*/scen/**', r => r.abort()) });   // the first search done
    await p2.waitForTimeout(300);   // the results' fade back from the searching dim, as above
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
