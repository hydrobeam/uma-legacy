import { PAGE_URL, check, open, settle, audit, contrastCheck } from './helpers.mjs';

export default async function dark(b) {
  // dark mode: the system's preference until the header button is used, then the visitor's choice, kept in this browser
  // (the loop planner's colours in dark mode are audited in `contrast`)
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
    .catch(() => {
      // the change event arrives asynchronously; the check below reports a timeout
    });
  check('dark: … and follows the system when it changes', (await theme()) === 'light');
  await pg.emulateMedia({ colorScheme: 'dark' });
  await pg
    .waitForFunction(() => document.documentElement.getAttribute('data-theme') === 'dark', null, { timeout: 3000 })
    .catch(() => {
      // the change event arrives asynchronously; on a timeout the theme is still light, the click below makes it dark
      // and the next check fails
    });
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
  if (b.browserType().name() === 'chromium') contrastCheck('dark: /about', [await audit(pg)]);
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
}
