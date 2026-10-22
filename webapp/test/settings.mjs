import { check, open, settle } from './helpers.mjs';

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
      } catch {}
    },
  });
  check(
    'saved settings load: the uma marked not owned shows not owned',
    await pg.evaluate(() =>
      [...document.querySelectorAll('#roster .ut[data-uma="1001"]')].every((t) => t.dataset.s === 'out'),
    ),
  );
  check(
    '… card ownership and rank too',
    (await pg.evaluate(() => document.querySelectorAll('#cards .ct[aria-pressed="false"]').length === 1)) &&
      (await pg.getAttribute('#rank', 'data-value')) === '0.20,0.70,0.10',
  );
  // changes persist across a reload
  await pg.locator('#roster .ut .up').nth(3).click();
  await settle(pg);
  await pg.reload();
  await settle(pg);
  check(
    'a change survives a reload',
    (await pg.locator('#roster .ut[data-s="out"]').count()) >=
      1 + (await pg.locator('#roster .ut[data-uma="1001"]').count()),
  );
  // export, then import the same text
  await pg.click('#ex-open');
  const text = await pg.inputValue('#ex-text');
  await pg.fill('#ex-text', text);
  await pg.click('#ex-load');
  check(
    'export then import: settings load',
    (await pg.locator('#ex-msg').innerText()) === 'Settings loaded.' && JSON.parse(text).settings.era === 'global',
  );
  // a setting of the wrong shape (a hand edit) is left out and named; the rest loads, and the page still loads after it
  // (the wrong value used to be saved first, and every later load failed on it)
  const bad = JSON.parse(text);
  bad.settings.targets = 5;
  bad.settings.rank = '0.50,0.45,0.05';
  await pg.fill('#ex-text', JSON.stringify(bad));
  await pg.click('#ex-load');
  check(
    'import: a malformed setting is named and left as it was, the others load',
    /Not understood, so left as they were: targets\.$/.test(await pg.locator('#ex-msg').innerText()) &&
      (await pg.locator('#targets .tgt').count()) === 8 &&
      (await pg.getAttribute('#rank', 'data-value')) === '0.50,0.45,0.05',
    await pg.locator('#ex-msg').innerText(),
  );
  await pg.click('#ex-close');
  await settle(pg);
  await pg.reload();
  await settle(pg);
  check(
    '… and the page loads after it',
    (await pg.locator('#targets .tgt').count()) === 8 && (await pg.locator('#results .loop').count()) > 0,
  );
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
      } catch {}
    },
  });
  check(
    'Grand Masters saved, Global data active: Unity Cup is used (its golds and rank)',
    (await p3.getAttribute('#x-scen', 'data-value')) === 'unity' &&
      (await p3.evaluate(() => window.__runs.at(-1).scenario === 'unity')) &&
      (await p3.getAttribute('#rank', 'data-value')) === '0.50,0.45,0.05' &&
      (await p3.getAttribute('#g-20049', 'data-g')) === '2',
  );
  await p3.context().close();
}
