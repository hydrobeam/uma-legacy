import { PAGE_URL, check, artless, open, settle, dialog, AUDIT, still } from './helpers.mjs';

export default async function dark(b) {
  // dark mode: the system's preference until the header button is used, then the visitor's choice, kept in this browser
  const pg = await open(b, { scheme: 'dark' });
  const theme = () => pg.evaluate(() => document.documentElement.getAttribute('data-theme'));
  check(
    'dark: with no saved choice the page follows a dark system preference',
    (await theme()) === 'dark' &&
      (await pg.evaluate(
        () =>
          getComputedStyle(document.body).backgroundColor !== 'rgb(243, 245, 248)' &&
          document.getElementById('theme-btn').getAttribute('aria-pressed') === 'true',
      )),
  );
  await pg.emulateMedia({ colorScheme: 'light' });
  await pg
    .waitForFunction(() => document.documentElement.getAttribute('data-theme') === 'light', null, { timeout: 3000 })
    .catch(() => {}); // the change event arrives asynchronously
  check('dark: … and follows the system when it changes', (await theme()) === 'light');
  await pg.emulateMedia({ colorScheme: 'dark' });
  await pg
    .waitForFunction(() => document.documentElement.getAttribute('data-theme') === 'dark', null, { timeout: 3000 })
    .catch(() => {});
  if (b.browserType().name() === 'chromium') {
    await still(pg); // the results' fade back from the searching dim
    const fails = [...(await pg.evaluate(AUDIT))];
    await pg.click('#x-scen');
    fails.push(...(await pg.evaluate(AUDIT)));
    await pg.keyboard.press('Escape');
    await pg.click('#rank-q');
    fails.push(...(await pg.evaluate(AUDIT)).filter((x) => /rtab|rfoot|gpop/.test(x)));
    await pg.keyboard.press('Escape');
    await pg.locator('.deck .dc').first().click();
    fails.push(...(await pg.evaluate(AUDIT)).filter((x) => /cpop|sw|gtl|ctt/.test(x)));
    await pg.keyboard.press('Escape');
    await pg.locator('.loop .lacts button', { hasText: 'Race schedule' }).first().click();
    await dialog(pg, 'sched-dlg');
    fails.push(...(await pg.evaluate(AUDIT)));
    await pg.locator('#sched-dlg .stab').last().click();
    fails.push(...(await pg.evaluate(AUDIT)));
    await pg.keyboard.press('Escape');
    await pg.locator('.loop .lacts button', { hasText: 'Affinity' }).first().click();
    await dialog(pg, 'why-dlg');
    fails.push(...(await pg.evaluate(AUDIT)));
    await pg.keyboard.press('Escape');
    {
      const shown = artless(fails);
      check(
        'dark: every text run meets WCAG AAA (page, scenario picker, rank popup, card popup, schedule, affinity)',
        !shown.length,
        shown.slice(0, 5).join(' | '),
      );
    }
  }
  await pg.click('#theme-btn');
  check(
    'dark: the header button switches to light and remembers it',
    (await theme()) === 'light' && (await pg.evaluate(() => localStorage.getItem('llp:theme') === 'light')),
  );
  await pg.reload();
  await settle(pg);
  check('dark: … the choice outlives a reload and beats the system preference', (await theme()) === 'light');
  await pg.emulateMedia({ colorScheme: 'light' });
  await pg.emulateMedia({ colorScheme: 'dark' });
  await pg.waitForTimeout(300);
  check('dark: … and a system change no longer overrides it', (await theme()) === 'light');
  // the status box repaints with the page, not after it (its colour transitions wait out the switch)
  const box = () => pg.evaluate(() => getComputedStyle(document.querySelector('.rhead')).backgroundColor);
  const bLight = await box();
  await pg.click('#theme-btn');
  const bNow = await box();
  await pg.waitForTimeout(900);
  const bLater = await box();
  check('dark: the button switches back to dark', (await theme()) === 'dark');
  check(
    'dark: … the status box takes its dark colour at once',
    bNow === bLater && bNow !== bLight,
    `${bLight} -> ${bNow} -> ${bLater}`,
  );
  await pg.goto(PAGE_URL.replace(/\/?$/, '/') + 'about');
  await pg.waitForLoadState('load');
  check('dark: /about takes the same choice', (await theme()) === 'dark');
  if (b.browserType().name() === 'chromium') {
    const f = await pg.evaluate(AUDIT);
    check('dark: /about meets WCAG AAA', !f.length, f.slice(0, 5).join(' | '));
  }
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
}
