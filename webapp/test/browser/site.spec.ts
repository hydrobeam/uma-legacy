// The site around the planners: the header's page tabs, /about's folded sections and its links into them, and the
// theme (the system's preference until the header button is used, then the visitor's choice, kept in this browser).
import { expect, test } from './fixtures.ts';
import { loopSettled, openLoop } from './pages.ts';

const PAGES = ['Career planner', 'Loop planner', 'How this works'];

test('header tabs lead to /about and back, the current page marked', async ({ page }) => {
  await openLoop(page);
  const nav = page.getByRole('navigation', { name: 'Pages' });
  const current = nav.locator('[aria-current="page"]');
  await expect(nav.getByRole('link')).toHaveText(PAGES);
  await expect(current).toHaveText('Loop planner');
  await nav.getByRole('link', { name: 'How this works', exact: true }).click();
  await expect(page).toHaveURL(/\/about$/);
  await expect(page.getByRole('main').getByRole('heading', { level: 2 })).toHaveText(/How this works/i);
  await expect(nav.getByRole('link')).toHaveText(PAGES);
  await expect(current).toHaveText('How this works');
  await nav.getByRole('link', { name: 'Loop planner', exact: true }).click();
  await expect(page).toHaveURL(/\/loop$/);
});

test('/about: folded sections open to their text; a contents list links every heading', async ({ page }) => {
  await page.goto('/about');
  const main = page.getByRole('main');
  const spec = main.locator('details').first();
  const cspec = main.locator('details').nth(1);
  await expect(spec).not.toHaveAttribute('open');
  await expect(main).not.toHaveText(/Model\./, { useInnerText: true });
  await spec.locator('summary').click();
  await expect(spec).toHaveText(/Model\./, { useInnerText: true });
  await expect(main).toHaveText(/Version v?\S+/, { useInnerText: true });
  await expect(cspec).not.toHaveAttribute('open');
  await cspec.locator('summary').click();
  for (const step of [/C1\. Legal careers/, /C8b\. Which careers are searched exactly/, /The Comparison panel/])
    await expect(cspec).toHaveText(step, { useInnerText: true });
  const toc = await page.evaluate(() => {
    const links = [...document.querySelectorAll('main nav[aria-label="On this page"] a')];
    return {
      links: links.length,
      ids: document.querySelectorAll('main h3[id]').length,
      headings: document.querySelectorAll('main h3').length,
      linked: links.every((a) => document.getElementById((a.getAttribute('href') ?? '').slice(1))?.tagName === 'H3'),
    };
  });
  expect(toc.links).toBeGreaterThanOrEqual(10);
  expect(toc).toEqual({ links: toc.headings, ids: toc.headings, headings: toc.headings, linked: true });
});

test('/about: a link to an "in detail" section opens that Show box alone', async ({ page }) => {
  await page.goto('/about#career-planner');
  const main = page.getByRole('main');
  const spec = main.locator('details').first();
  await expect(main.locator('details').nth(1)).toHaveAttribute('open');
  await expect(spec).not.toHaveAttribute('open');
  await page.getByRole('navigation', { name: 'On this page' }).locator('a[href="#model"]').click();
  await expect(spec).toHaveAttribute('open');
});

test.describe('theme', () => {
  test.use({ colorScheme: 'dark' });

  test('follows the system until the button is used', async ({ page }) => {
    await page.goto('/about');
    const html = page.locator('html');
    await expect(html).toHaveAttribute('data-theme', 'dark');
    await expect(page.locator('body')).not.toHaveCSS('background-color', 'rgb(243, 245, 248)');
    await expect(page.locator('#theme-btn')).toHaveAttribute('aria-pressed', 'true');
    await page.emulateMedia({ colorScheme: 'light' });
    await expect(html).toHaveAttribute('data-theme', 'light');
    await page.emulateMedia({ colorScheme: 'dark' });
    await expect(html).toHaveAttribute('data-theme', 'dark');
  });

  test('the chosen theme is kept over reloads and the system, and the status box repaints at once', async ({
    page,
  }) => {
    await openLoop(page);
    const html = page.locator('html');
    await page.click('#theme-btn');
    await expect(html).toHaveAttribute('data-theme', 'light');
    expect(await page.evaluate(() => localStorage.getItem('llp:theme'))).toBe('light');
    await page.reload();
    await loopSettled(page);
    await expect(html).toHaveAttribute('data-theme', 'light');
    await page.emulateMedia({ colorScheme: 'light' });
    await page.emulateMedia({ colorScheme: 'dark' });
    // (a system change that wrongly took effect would have done so within a frame or two)
    await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
    await expect(html).toHaveAttribute('data-theme', 'light');
    // the status box takes its colour with the page, not after a transition
    const box = () =>
      page.evaluate(() => getComputedStyle(document.getElementById('rhead') ?? document.body).backgroundColor);
    const light = await box();
    await page.click('#theme-btn');
    await expect(html).toHaveAttribute('data-theme', 'dark');
    const now = await box();
    expect(now).not.toBe(light);
    await page.evaluate(() => Promise.all(document.getAnimations().map((a) => a.finished)));
    expect(await box()).toBe(now);
    await page.goto('/about');
    await expect(html).toHaveAttribute('data-theme', 'dark');
  });
});
