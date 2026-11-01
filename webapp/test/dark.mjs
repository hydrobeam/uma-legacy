import { expect } from 'playwright/test';
import { PAGE_URL, check, open, settle } from './helpers.mjs';

export default async function dark(b) {
  // dark mode: the system's preference until the header button is used, then the visitor's choice, kept in this browser
  const pg = await open(b, { scheme: 'dark' });
  const html = pg.locator('html');
  // the page's theme; `timeout`: how long a change the system's preference starts may take (its event is asynchronous)
  const themeIs = (theme, what, timeout) => expect.soft(html, what).toHaveAttribute('data-theme', theme, { timeout });
  const follows = 'dark: with no saved choice the page follows a dark system preference';
  await themeIs('dark', follows);
  await expect
    .soft(pg.locator('body'), `${follows}: its colours`)
    .not.toHaveCSS('background-color', 'rgb(243, 245, 248)');
  await expect.soft(pg.locator('#theme-btn'), `${follows}: the button pressed`).toHaveAttribute('aria-pressed', 'true');
  await pg.emulateMedia({ colorScheme: 'light' });
  await themeIs('light', 'dark: … and follows the system when it changes', 3000);
  await pg.emulateMedia({ colorScheme: 'dark' });
  // (before the click below, which then makes it light)
  await themeIs('dark', 'dark: … and back when the system changes back', 3000);
  await pg.click('#theme-btn');
  const remembers = 'dark: the header button switches to light and remembers it';
  await themeIs('light', remembers);
  check(`${remembers}: stored`, await pg.evaluate(() => localStorage.getItem('llp:theme') === 'light'));
  await pg.reload();
  await settle(pg);
  await themeIs('light', 'dark: … the choice outlives a reload and beats the system preference');
  await pg.emulateMedia({ colorScheme: 'light' });
  await pg.emulateMedia({ colorScheme: 'dark' });
  await pg.waitForTimeout(300); // time for a system change to (wrongly) take effect
  await themeIs('light', 'dark: … and a system change no longer overrides it');
  // the status box repaints with the page, not after it (its colour transitions wait out the switch)
  const box = () => pg.evaluate(() => getComputedStyle(document.querySelector('.rhead')).backgroundColor);
  const bLight = await box();
  await pg.click('#theme-btn');
  const bNow = await box();
  await pg.waitForTimeout(900);
  const bLater = await box();
  await themeIs('dark', 'dark: the button switches back to dark');
  check(
    'dark: … the status box takes its dark colour at once',
    bNow === bLater && bNow !== bLight,
    `${bLight} -> ${bNow} -> ${bLater}`,
  );
  await pg.goto(PAGE_URL.replace(/\/?$/, '/') + 'about');
  await themeIs('dark', 'dark: /about takes the same choice');
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
}
