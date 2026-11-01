import { expect } from 'playwright/test';
import { check, open, openEditor, settle } from './helpers.mjs';

export default async function settings(b) {
  // saved settings load: roster and cards per data set, the rest shared
  const pg = await open(b, {
    init: () => {
      try {
        if (sessionStorage.getItem('seeded')) return;
        sessionStorage.setItem('seeded', '1');
        localStorage.clear();
        localStorage.setItem(
          'llp:settings',
          JSON.stringify({
            v: 2,
            era: 'global',
            common: { rank: '0.20,0.70,0.10' },
            global: { roster: { 1001: 'out' }, cardsOff: { 30052: false } },
            gm: { roster: { 1002: 'out' } },
          }),
        );
      } catch {
        // storage blocked: the check below fails
      }
    },
  });
  check(
    'saved settings load: the uma marked not owned shows not owned',
    await pg.evaluate(() =>
      [...document.querySelectorAll('#roster .ut[data-uma="1001"]')].every((t) => t.dataset.s === 'out'),
    ),
  );
  await expect.soft(pg.locator('#cards .ct[aria-pressed="false"]'), '… card ownership too').toHaveCount(1);
  await expect.soft(pg.locator('#rank'), '… and rank').toHaveAttribute('data-value', '0.20,0.70,0.10');
  // changes persist across a reload
  await openEditor(pg, 'umas');
  await pg.locator('#roster .ut .up').nth(3).click();
  await settle(pg);
  await pg.reload();
  await settle(pg);
  check(
    'a change survives a reload',
    (await pg.locator('#roster .ut[data-s="out"]').count()) >=
      1 + (await pg.locator('#roster .ut[data-uma="1001"]').count()),
  );
  // export, then import it with the rank changed: the import sets it
  await pg.click('#ex-open');
  const text = await pg.inputValue('#ex-text');
  const other = JSON.parse(text);
  other.settings.rank = '0.50,0.45,0.05';
  await pg.fill('#ex-text', JSON.stringify(other));
  await pg.click('#ex-load');
  check('export: the rank as set', JSON.parse(text).settings.rank === '0.20,0.70,0.10');
  await expect.soft(pg.locator('#ex-msg'), 'export then import: settings load').toHaveText('Settings loaded.');
  await expect.soft(pg.locator('#rank'), '… the rank imported').toHaveAttribute('data-value', '0.50,0.45,0.05');
  // a setting of the wrong shape (a hand edit) is left out and named; the rest loads, and the page still loads after it
  // (the wrong value used to be saved first, and every later load failed on it)
  const bad = JSON.parse(text);
  bad.settings.targets = 5;
  await pg.fill('#ex-text', JSON.stringify(bad));
  await pg.click('#ex-load');
  await expect
    .soft(pg.locator('#ex-msg'), 'import: a malformed setting is named')
    .toHaveText(/Not understood, so left as they were: targets\.$/);
  await expect.soft(pg.locator('#targets > [data-g]'), '… and left as it was').toHaveCount(8);
  await expect.soft(pg.locator('#rank'), '… the others load').toHaveAttribute('data-value', '0.20,0.70,0.10');
  await pg.click('#ex-close');
  await settle(pg);
  await pg.reload();
  await settle(pg);
  await expect.soft(pg.locator('#targets > [data-g]'), '… and the page loads after it: its targets').toHaveCount(8);
  await expect.soft(pg.locator('#results .loop').first(), '… and its results').toBeAttached();
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
  const p3 = await open(b, {
    init: () => {
      try {
        if (sessionStorage.getItem('seeded')) return;
        sessionStorage.setItem('seeded', '1');
        localStorage.clear();
        localStorage.setItem(
          'llp:settings',
          JSON.stringify({
            v: 2,
            era: 'global',
            common: {
              scen: 'grandmasters',
              rank: '0.20,0.70,0.10',
              targets: [{ group: 20049 }, { group: 20046 }, { group: 20159 }],
            },
            global: {},
            gm: {},
          }),
        );
      } catch {
        // storage blocked: the check below fails
      }
    },
  });
  const gm = 'Grand Masters saved, Global data active: Unity Cup is used';
  await expect.soft(p3.locator('#x-scen'), gm).toHaveAttribute('data-value', 'unity');
  check(`${gm}: in the search`, await p3.evaluate(() => window.__runs.at(-1).scenario === 'unity'));
  await expect.soft(p3.locator('#rank'), `${gm}: its rank`).toHaveAttribute('data-value', '0.50,0.45,0.05');
  await expect.soft(p3.locator('#g-20049'), `${gm}: its golds`).toHaveAttribute('data-g', '2');
  await p3.context().close();
}
