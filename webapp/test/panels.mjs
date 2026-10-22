import { check, open, settle, } from './helpers.mjs';

export default async function panels(b) {
  const pg = await open(b);
  // the worker's replies are held until the last switch: a search an Owned switch starts would otherwise end, under
  // load, before the next step, and its new results close the panel that step clicks in
  await pg.evaluate(() => { window.__hold = true; });
  const dc = pg.locator('.deck .dc').first(); await dc.click();
  const href = await pg.locator('.cpop .gtl').getAttribute('href');
  check('deck card panel: GameTora card link', /^https:\/\/gametora\.com\/umamusume\/supports\/\d+-[a-z0-9-]+$/.test(href || ''), href);
  const side = await pg.evaluate(() => { const c = document.querySelector('.deck .dc').getBoundingClientRect(), p = document.querySelector('.cpop').getBoundingClientRect(); return { beside: p.left >= c.right || p.right <= c.left, top: Math.abs(p.top - c.top) }; });
  check('deck card panel opens beside the card (desktop), not over the deck below', side.beside && side.top <= 2, JSON.stringify(side));
  const cid = await dc.getAttribute('data-cid');
  await pg.locator('.cpop .sw').click();
  check('deck card Owned switch updates the card picker', await pg.evaluate((_id) => [...document.querySelectorAll('#cards .ct')].some(t => t.querySelector('img') && t.getAttribute('aria-pressed') === 'false'), cid));
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
  await pg.locator('.cpop .sw').click(); await pg.keyboard.press('Escape'); await pg.evaluate(() => window.__release()); await settle(pg);
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
}
