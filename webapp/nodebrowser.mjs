// Browser regression suite for the built page (webapp/dist, served on a local port; LLP_URL tests another address).
// Node + Playwright (local or global install). LLP_SHOTS=dir saves pictures of the scenario button and picker there.
//   node webapp/nodebrowser.mjs                 every test, Chromium and Firefox
//   node webapp/nodebrowser.mjs chromium roster only the named browsers / tests (any order)
// Exit code 1 if any check fails. Expectations come from the page's own embedded data where possible, so a data
// update does not break the suite; the one fixed number is the Global default search (1 in 139, R cards off, Unity Cup).
import { createRequire } from 'module';
import { execSync } from 'child_process';
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
const check = (name, ok, detail = '') => { if (ok) passed++; else failed++; console.log(`${ok ? '  ok  ' : '  FAIL'} ${name}${detail ? ' · ' + detail : ''}`); };

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
    const off = await pg.evaluate(() => Math.round(document.querySelector('#results .loop').getBoundingClientRect().top - document.querySelector('.rail .panel').getBoundingClientRect().top));
    check('results start level with the settings column', Math.abs(off) <= 1, off + ' px');
    check('no button; status bar present', (await pg.locator('#run').count()) === 0 && (await pg.locator('#run-note').count()) === 1);
    // the scenario sets the training-hint rate: switching it searches again with the scenario in the job, and the
    // choice is kept (the default setup's odds barely move: its decks get the targets from events, not training hints)
    const n0 = await pg.evaluate(() => window.__runs.length);
    check('the first search is a Unity Cup job', await pg.evaluate(() => window.__runs.at(-1).scenario === 'unity'));
    await pickScen(pg, 'ura'); await settle(pg);
    check('scenario switch searches again with the URA job', await pg.evaluate((n) => window.__runs.length > n && window.__runs.at(-1).scenario === 'ura', n0));
    // Unity Cup's golds follow the scenario: off under URA, back on under Unity, and the 8/8 example takes them too
    check('URA drops the Unity golds', (await pg.inputValue('#g-20049')) === '0' && (await pg.inputValue('#g-20046')) === '0');
    await pg.click('#t-preset');
    check('8/8 example under URA: eight targets, no guaranteed golds', (await pg.locator('#targets .tgt').count()) === 8 && (await pg.inputValue('#g-20049')) === '0');
    await settle(pg);
    await pg.reload(); await settle(pg);
    check('scenario survives a reload', (await scen(pg)) === 'ura' && (await pg.getAttribute('#x-scen', 'aria-label')) === 'Scenario: URA Finale');
    await pickScen(pg, 'unity');
    check('Unity Cup marks No Stopping Me! and It\'s On! as guaranteed golds', (await pg.inputValue('#g-20049')) === '2' && (await pg.inputValue('#g-20046')) === '2');
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
    check('picking Grand Concert sets the rank to SS', (await pg.inputValue('#rank')) === '0.20,0.70,0.10' && await pg.evaluate(() => window.__runs.at(-1).star_dist.join() === '0.2,0.7,0.1'));
    await pickScen(pg, 'unity'); await settle(pg);
    check('picking Unity Cup sets it back to below SS', (await pg.inputValue('#rank')) === '0.50,0.45,0.05');
    await pg.selectOption('#rank', '0.20,0.70,0.10'); await settle(pg); await pickScen(pg, 'unity'); await pg.waitForTimeout(400);
    check('a rank changed by hand sticks (re-picking the scenario leaves it)', (await pg.inputValue('#rank')) === '0.20,0.70,0.10' && await pg.evaluate(() => window.__runs.at(-1).star_dist.join() === '0.2,0.7,0.1'));
    await pg.reload(); await settle(pg);
    check('… and survives a reload', (await pg.inputValue('#rank')) === '0.20,0.70,0.10' && (await scen(pg)) === 'unity');
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
    await p2.route('**/img/scen/**', r => r.abort()); await p2.goto(URL); await p2.waitForTimeout(800); await p2.click('#x-scen');
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
    const sep = await pg.evaluate(() => { const c = document.getElementById('cards'), r = c.previousElementSibling.getBoundingClientRect(), cs = getComputedStyle(c);
      return { gap: Math.round(c.getBoundingClientRect().top - r.bottom), pad: parseFloat(cs.paddingTop) }; });
    check('the card list scrolls apart from its search row (gap like the roster\'s), its tiles\' rings and focus outline inside it', sep.gap >= 10 && sep.pad >= 5, JSON.stringify(sep));
    check('default card sort: release, newest first', (await pg.locator('#cs').innerText()) === 'Release ↓');
    const tile = pg.locator('#cards .ct').first(); const img = await tile.evaluate(t => { t.__img = t.querySelector('img'); return !!t.__img; });
    await tile.click();
    check('toggling a card keeps its image element (no flicker)', img && await tile.evaluate(t => t.querySelector('img') === t.__img && t.getAttribute('aria-pressed') === 'false'));
    check('the count line follows', /1 not owned/.test(await pg.locator('#c-count').innerText()), await pg.locator('#c-count').innerText());
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
    const mb = pg.locator('.loop').first().locator('.mem .memb').nth(1); await mb.click(); await pg.waitForTimeout(100);
    check('member panel has one Owned switch', (await pg.locator('.cpop input[role=switch]').count()) === 1);
    await pg.locator('.cpop .sw').click(); await pg.waitForTimeout(100);
    check('member Owned switch updates the roster and greys her', (await pg.locator('#roster .ut[data-s="out"]').count()) === 1 && (await pg.locator('.mem.off').count()) > 0);
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
    await pg.locator('#sched-dlg .stab').nth(4).click(); check('All four shows the comparison table', (await pg.locator('#sched-dlg table.sched').count()) > 0);
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
    const H = await pg.evaluate(() => document.documentElement.scrollHeight); let vis = true;
    for (const f of [0, 0.5, 1]) { await pg.evaluate((y) => scrollTo(0, y), f * H); await pg.waitForTimeout(60);
      vis = vis && await pg.evaluate(() => { const r = document.getElementById('run-note').getBoundingClientRect(); return r.top >= 0 && r.bottom <= innerHeight; }); }
    check('phone: the status bar stays on screen', vis);
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
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async gm(b) {
    const pg = await open(b, { init: () => { try { if (!sessionStorage.getItem('gm')) { sessionStorage.setItem('gm', '1'); localStorage.setItem('llp:era', 'gm'); } } catch {} } });
    await pg.reload(); await settle(pg);
    const umas = (await pg.evaluate(() => fetch('data-gm.json').then(r => r.json()))).umas; const nOut = umas.reduce((a, u) => a + Math.max(1, u.outfits.length), 0);
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
    const st = await pg.evaluate(() => ({ keys: Object.keys(localStorage).filter(k => k.startsWith('llp:')), s: JSON.parse(localStorage.getItem('llp:settings')) }));
    check('migration: one llp:settings object, old keys removed', st.keys.length === 1 && st.keys[0] === 'llp:settings' && st.s.v === 2, st.keys.join(','));
    check('migration: per-era roster kept apart', st.s.global.roster['1001'] === 'out' && st.s.gm.roster['1002'] === 'out');
    check('migration: the uma marked not owned shows not owned', await pg.evaluate(() => [...document.querySelectorAll('#roster .ut[data-uma="1001"]')].every(t => t.dataset.s === 'out')));
    check('migration: card ownership kept', await pg.evaluate(() => document.querySelectorAll('#cards .ct[aria-pressed="false"]').length === 1));
    check('migration: old preset gold fixed, hand-set odds dropped', JSON.stringify(st.s.common.targets) === JSON.stringify([{ group: 20166, guaranteed: 0 }, { group: 20159, guaranteed: 0 }]));
    check('migration: old star mix remapped', (await pg.inputValue('#rank')) === '0.50,0.45,0.05');
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
      && await p3.evaluate(() => window.__runs.at(-1).scenario === 'unity') && (await p3.inputValue('#rank')) === '0.50,0.45,0.05' && (await p3.inputValue('#g-20049')) === '2');
    await p3.context().close();
    check('v1 settings: R cards reset to off, other settings kept', !(await p2.locator('#x-rcards').isChecked()) && (await p2.inputValue('#rank')) === '0.20,0.70,0.10');
    await p2.context().close();
  },
  async contrast(b) {
    if (b.browserType().name() !== 'chromium') return;   // one engine is enough for colours
    const pg = await open(b);
    await pg.locator('#roster .ut .up').nth(1).click(); await pg.locator('#cards .ct').nth(1).click(); await settle(pg);
    const fails = [...await pg.evaluate(AUDIT)];
    await pg.click('#x-scen'); fails.push(...await pg.evaluate(AUDIT)); await pg.keyboard.press('Escape');
    await pg.locator('.deck .dc').first().click(); fails.push(...(await pg.evaluate(AUDIT)).filter(x => /cpop|sw|gtl|ctt/.test(x))); await pg.keyboard.press('Escape');
    await pg.locator('.loop .lacts button', { hasText: 'Race schedule' }).first().click(); await pg.waitForTimeout(200); fails.push(...await pg.evaluate(AUDIT));
    await pg.locator('#sched-dlg .stab').nth(4).click(); fails.push(...await pg.evaluate(AUDIT)); await pg.keyboard.press('Escape');
    await pg.locator('.loop .lacts button', { hasText: 'Affinity' }).first().click(); await pg.waitForTimeout(200); fails.push(...await pg.evaluate(AUDIT));
    check('contrast: every text run meets WCAG AAA', !fails.length, fails.slice(0, 5).join(' | '));
    await pg.context().close();
    const p2 = await b.newContext().then(c => c.newPage()); await p2.route('**/img/scen/**', r => r.abort()); await p2.goto(URL); await p2.waitForTimeout(800);
    await p2.click('#x-scen'); const bf = (await p2.evaluate(AUDIT)).filter(x => /^b\./.test(x) || /scen|stile|slogo|snm|ssub/.test(x));
    check('contrast: scenario badges (no logo files) meet WCAG AAA', !bf.length, bf.join(' | '));
    await p2.context().close();
  },
};

const pick = Object.keys(TESTS).filter(t => !argv.some(a => a in TESTS) || argv.includes(t));
for (const name of BROWSERS) {
  const b = await pw[name].launch();
  for (const t of pick) { console.log(`[${name}] ${t}`); try { await TESTS[t](b); } catch (e) { failed++; console.log('  FAIL ' + t + ' threw: ' + String(e).split('\n')[0]); } }
  await b.close();
}
console.log(`\n${passed} passed, ${failed} failed`);
process.exit(failed ? 1 : 0);
