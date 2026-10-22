import { PAGE_URL, check, PILLS, SIDEWAYS, still } from './helpers.mjs';

export default async function careerphone(b) {
  // the career page on a 375 px phone (iPhone 13 mini) with the sample veterans' answer: nothing scrolls sideways (page or dialogs), the
  // setup rows inside their panel, the pill tracks flush, the pick in Options shaded as one card
  const chromium = b.browserType().name() === 'chromium';
  const ctx = await b.newContext({ viewport: { width: 375, height: 812 }, ...(chromium ? { isMobile: true, hasTouch: true } : {}) });
  const pg = await ctx.newPage(); pg.errors = [];
  pg.on('pageerror', e => pg.errors.push(String(e))); pg.on('console', m => m.type() === 'error' && !/downloadable font/.test(m.text()) && pg.errors.push(m.text()));
  await pg.goto(PAGE_URL.replace(/\/?$/, '/') + 'career');
  await pg.waitForFunction(() => /Try sample veterans/.test(document.getElementById('run-act')?.textContent || ''), null, { timeout: 180000 });
  await pg.click('#run-act');
  await pg.waitForFunction(() => /Planned in/.test(document.getElementById('run-note').textContent) && document.querySelector('.ud .udtabs'), null, { timeout: 300000 });
  const inside = await pg.evaluate(() => { const p = document.querySelector('.rbbar').getBoundingClientRect();
    return [...document.querySelectorAll('.rbbar .setlist > *, .rbbar .setlist .seg, .rbbar .setlist .scenbtn, .rbbar .lptgt .adder')].filter(e => e.getBoundingClientRect().width)
      .filter(e => { const r = e.getBoundingClientRect(); return r.left < p.left || r.right > p.right; }).map(e => e.id || e.className); });
  check('career phone: no sideways scroll; the setup rows and the add-a-skill box sit inside their panel', !(await pg.evaluate(SIDEWAYS)).length && !inside.length
    && (await pg.locator('.rbbar .lptgt .adder input').boundingBox()).width > 200, JSON.stringify([await pg.evaluate(SIDEWAYS), inside]));
  await pg.locator('.udtab').nth(1).click();
  const pills = await pg.evaluate(PILLS);
  check('career phone: the pill tracks (Game data, rank, her card\'s tabs) are filled flush by their segments', !pills.length && await pg.locator('.udtabs').count() === 1, pills.slice(0, 4).join(' | '));
  const pick = await pg.evaluate(() => { const tr = document.querySelector('.alts .optab tbody tr.on'); if (!tr) return null; const th = tr.querySelector('th');
    return { bg: getComputedStyle(tr).backgroundColor, cell: getComputedStyle(th).backgroundColor, wide: th.getBoundingClientRect().width >= tr.clientWidth - parseFloat(getComputedStyle(tr).paddingLeft) - parseFloat(getComputedStyle(tr).paddingRight) - 4 }; });   // the card's content width (it has inner padding)
  check('career phone: Career choices shades the planner\'s pick as one card, its career the card\'s full width', pick && pick.bg !== 'rgba(0, 0, 0, 0)' && pick.cell === 'rgba(0, 0, 0, 0)' && pick.wide, JSON.stringify(pick));
  const side = [];
  const shut = () => pg.evaluate(() => document.querySelectorAll('dialog[open]').forEach(d => { d.close(); }));
  for (const [open, label] of [[() => pg.click('#v-toggle'), 'veterans'], [() => pg.click('#o-umas'), 'umas'], [() => pg.click('#o-cards'), 'cards'],
    [() => pg.locator('#tree [data-k]').first().click(), 'node'], [() => pg.locator('.udodds').first().click(), 'odds']]) {
    await open(); await pg.waitForSelector('dialog[open]'); await still(pg);
    side.push(...(await pg.evaluate(SIDEWAYS)).map(x => label + ': ' + x));
    if (label === 'veterans') side.push(...(await pg.evaluate(PILLS)).map(x => label + ': ' + x));
    await shut(); }
  check('career phone: nothing scrolls sideways in your veterans (their Yours / Borrowed pills flush), Umas, Support cards, the node popup or the odds', !side.length, side.slice(0, 4).join(' | '));
  const tree = await pg.evaluate(() => { const f = [...document.querySelectorAll('#tree .pf')].map(e => e.getBoundingClientRect());
    return { h: Math.round(document.getElementById('tree').getBoundingClientRect().height), side: f.length === 2 && Math.abs(f[0].top - f[1].top) < 1 && f[1].left >= f[0].right - 1 }; });
  check('career phone: the tree is compact, the two legacy boxes side by side', tree.side && tree.h < 560, JSON.stringify(tree));
  // a return visit with veterans and no fold chosen: the setup opens folded, the tree on the first screen; a fold chosen
  // (here: opened) is kept
  const back = async () => { await pg.reload(); await pg.waitForFunction(() => /Planned in/.test(document.getElementById('run-note').textContent), null, { timeout: 300000 }); };
  await back();
  const ret = await pg.evaluate(() => ({ folded: document.querySelector('.rbbar').classList.contains('folded'), sum: !document.getElementById('set-sum').hidden,
    top: Math.round(document.getElementById('tree').getBoundingClientRect().top + scrollY) }));
  await pg.click('#set-fold'); await back();
  check('career phone: back with veterans, the setup opens folded and the tree starts on the first screen; opened, it stays open', ret.folded && ret.sum && ret.top < 812
    && !(await pg.locator('.rbbar.folded').count()), JSON.stringify(ret));
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await ctx.close();
}
