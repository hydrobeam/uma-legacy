import { check, artCheck, open, settle, dialog, until, PILLS, SIDEWAYS } from './helpers.mjs';

export default async function phone(b) {
  const pg = await open(b, { w: 375, h: 812, mobile: true });
  const small = await pg.evaluate(() => [...document.querySelectorAll('.lp .rbbar button, .lp .rbbar select, #rhead button, #rhead select, .loop .lacts button, .mem .memb')]
    .filter(e => e.offsetParent && !e.closest('.roster, .cards')).map(e => [e.id || e.className || e.tagName, Math.round(e.getBoundingClientRect().height)]).filter(([, h]) => h < 44));
  check('phone: buttons, selects and search boxes are at least 44 px high', !small.length, JSON.stringify(small.slice(0, 6)));
  // results first: above the settings, and first in the document (reading and Tab order), the result in the first screen
  const order = await pg.evaluate(() => { const c = document.getElementById('rcol'), r = document.querySelector('.lp .rbbar'), o = document.querySelector('#results .loop .odds').getBoundingClientRect();
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
  const boxes = []; for (const id of ['roster', 'cards']) { await pg.editor('#' + id); await pg.waitForTimeout(100);
    boxes.push(await pg.evaluate((id) => { const e = document.getElementById(id); return { h: e.clientHeight / innerHeight, more: e.classList.contains('more') }; }, id)); }
  const part = await pg.evaluate(() => { const s = new Set(); let over = 0; for (const t of document.querySelectorAll('#cards .ct')) { const c = t.querySelector('.cav').getBoundingClientRect();
    s.add(Math.round(c.width)); if (c.width > t.getBoundingClientRect().width + 0.5) over++; } return { sizes: [...s], over }; });
  check('phone: every card picture is the same size and inside its tile, loaded or not', part.sizes.length === 1 && !part.over, JSON.stringify(part));
  check('phone: the roster and card boxes fit in their dialogs and show that they scroll', boxes.every(x => x.h <= 0.8 && x.more), JSON.stringify(boxes));
  await pg.evaluate(() => { document.querySelectorAll('dialog[open]').forEach(d => { d.close(); }); scrollTo(0, 0); });
  await pg.click('#x-scen');
  check('phone: the open scenario picker fits the screen', await pg.evaluate(() => { const r = document.getElementById('scen-menu').getBoundingClientRect();
    return r.width > 0 && r.left >= 0 && r.right <= innerWidth && document.documentElement.scrollWidth <= document.documentElement.clientWidth; }));
  const ph = await pg.evaluate(() => [...document.querySelectorAll('#scen-menu .stile')].map(t => { const r = t.getBoundingClientRect(), s = t.querySelector('.ssub').getBoundingClientRect();
    return Math.round(r.width) + 'x' + Math.round(r.height) + ' line ' + Math.round(s.height); }));
  artCheck('phone: the scenario tiles are the same size, each logo + one line', new Set(ph.map(x => x.split(' ')[0])).size === 1 && ph.every(x => +x.split(' ').at(-1) <= 22), ph.join(' | '));
  await pg.keyboard.press('Escape');
  await pg.click('#rank-q');
  check('phone: the open lineage-rank popup fits the screen', await pg.evaluate(() => { const r = document.getElementById('rank-note').getBoundingClientRect();
    return r.width > 0 && r.left >= 0 && r.right <= innerWidth && document.documentElement.scrollWidth <= document.documentElement.clientWidth; }));
  await pg.keyboard.press('Escape');
  await pg.locator('.loop .lacts button', { hasText: 'Race schedule' }).first().click(); await dialog(pg, 'sched-dlg');
  check('phone: agenda stacks without the grey Junior cells', (await pg.locator('#sched-dlg .ag-slot.pre:visible').count()) === 0);
  await pg.evaluate(() => { const d = document.querySelector('#sched-dlg .dlg'); d.scrollTo(0, d.scrollHeight); });
  const closeVis = () => pg.evaluate(() => { const b = [...document.querySelectorAll('dialog[open] .dhead button')].find(x => x.textContent === 'Close'), r = b.getBoundingClientRect(); return r.top >= 0 && r.bottom <= innerHeight; });
  check('phone: the schedule\'s Close stays on screen at the bottom of the dialog', await closeVis());
  await pg.locator('#sched-dlg .stab').last().click();
  check('phone: the All three table fits the 375 px screen', await pg.evaluate(() => [...document.querySelectorAll('#sched-dlg .tablewrap')].every(w => w.scrollWidth <= w.clientWidth)));
  await pg.keyboard.press('Escape');
  await pg.locator('.loop .lacts button', { hasText: 'Affinity' }).first().click(); await dialog(pg, 'why-dlg');
  await pg.evaluate(() => { const d = document.querySelector('#why-dlg .dlg'); d.scrollTo(0, d.scrollHeight); });
  check('phone: the affinity\'s Close stays on screen too', await closeVis());
  await pg.keyboard.press('Escape');
  await pg.click('#ex-open'); check('the export dialog\'s Close sits in the same header row as the others', await pg.evaluate(() => document.getElementById('ex-close').closest('.dhead') !== null)); await pg.keyboard.press('Escape');
  // the pill switches (Game data, lineage rank, Results): the chosen part fills its share of the track, flush
  await pg.evaluate(() => scrollTo(0, 0));
  const pills = await pg.evaluate(PILLS);
  check('phone: every pill track\'s segments fill it flush (full height, no gap at its ends or between them)', !pills.length && await pg.locator('.seg:visible').count() >= 3, pills.slice(0, 4).join(' | '));
  // nothing scrolls sideways with the editors and the result's popups open
  const side = [];
  for (const [sel, id, text] of [['#o-umas', 'set-umas'], ['#o-cards', 'set-cards'], ['.loop .lacts button', 'why-dlg', 'Affinity'], ['.loop .lacts button', 'odds-dlg', 'Odds']]) {
    const loc = text ? pg.locator(sel, { hasText: text }).first() : pg.locator(sel); await loc.scrollIntoViewIfNeeded(); await loc.click(); await dialog(pg, id);
    side.push(...(await pg.evaluate(SIDEWAYS)).map(x => id + ': ' + x)); await pg.evaluate(() => document.querySelectorAll('dialog[open]').forEach(d => { d.close(); })); }
  check('phone: nothing scrolls sideways with Umas, Support cards, Affinity or Odds open', !side.length, side.slice(0, 4).join(' | '));
  const swTouch = await pg.evaluate(() => [...document.querySelectorAll('.rhead .sw')].map(l => Math.round(l.getBoundingClientRect().height - 2 * parseFloat(getComputedStyle(l, '::before').top))));
  check('phone: the Auto-search and Deck per uma switches have a 44 px tall touch area', swTouch.length === 2 && swTouch.every(h => h >= 44), swTouch.join());
  await pg.setViewportSize({ width: 1280, height: 800 });   // the page reorders on its resize handler
  check('widened to desktop, the results go back beside (after) the settings', await until(pg, () => !!(document.querySelector('.lpside').compareDocumentPosition(document.getElementById('rcol')) & Node.DOCUMENT_POSITION_FOLLOWING)));
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
}
