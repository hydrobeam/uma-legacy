import { expect } from 'playwright/test';
import { PAGE_URL, check, open } from './helpers.mjs';

export default async function about(b) {
  // "How this works": a link in the header to its own page (/about), which links back
  const pg = await open(b);
  // the header is the same on every page: the three pages in one order, the page you are on marked
  const nav = pg.getByRole('navigation', { name: 'Pages' }),
    tabs = nav.getByRole('link'),
    current = nav.locator('[aria-current="page"]'),
    PAGES = ['Career planner', 'Loop planner', 'How this works'];
  const header = 'header: the same page tabs everywhere, the current page marked';
  await expect.soft(tabs, header).toHaveText(PAGES);
  await expect.soft(current, `${header}: the loop planner`).toHaveText('Loop planner');
  await nav.getByRole('link', { name: 'How this works', exact: true }).click();
  await pg.waitForLoadState(); // (the negative checks below read a page that has loaded)
  await expect.soft(pg, 'header link opens /about').toHaveURL(/\/about$/);
  await expect.soft(pg.locator('main.about h2'), 'header link opens /about: its heading').toHaveText(/How this works/i);
  await expect.soft(tabs, `${header}: /about`).toHaveText(PAGES);
  await expect.soft(current, `${header}: /about marked`).toHaveText('How this works');
  const main = pg.locator('main.about'),
    spec = pg.locator('main.about details.spec').first(),
    cspec = pg.locator('main.about details.spec').nth(1);
  const model = '/about: the detailed model starts folded and opens to the model text; the version shows';
  await expect.soft(spec, model).not.toHaveAttribute('open');
  await expect.soft(main, `${model}: no model text folded`).not.toHaveText(/Model\./, { useInnerText: true });
  await spec.locator('summary').click();
  await expect.soft(spec, `${model}: open`).toHaveText(/Model\./, { useInnerText: true });
  await expect.soft(main, `${model}: the version`).toHaveText(/Version v?\S+/, { useInnerText: true });
  const steps = '/about: the career planner in detail starts folded and opens to its steps';
  await expect.soft(cspec, steps).not.toHaveAttribute('open');
  await cspec.locator('summary').click();
  for (const step of [/C1\. Legal careers/, /C8b\. Which careers are searched exactly/, /The Comparison panel/])
    await expect.soft(cspec, `${steps}: ${step.source}`).toHaveText(step, { useInnerText: true });
  const toc = await pg.evaluate(() => {
    const a = [...document.querySelectorAll('main.about nav.toc a')];
    return {
      n: a.length,
      h: document.querySelectorAll('main.about h3[id]').length,
      all: document.querySelectorAll('main.about h3').length,
      ok: a.every((x) => document.getElementById(x.getAttribute('href').slice(1))?.tagName === 'H3'),
    };
  });
  check(
    '/about: a contents list links every section heading',
    toc.n === toc.h && toc.h === toc.all && toc.n >= 10 && toc.ok,
    JSON.stringify(toc),
  );
  // a link to an "in detail" section opens its Show box (from another page, or the contents)
  await pg.goto(PAGE_URL.replace(/\/?$/, '/'));
  await pg.goto(PAGE_URL.replace(/\/?$/, '/') + 'about#career-planner');
  await pg.waitForLoadState('load');
  const link = '/about: a link to an "in detail" section opens its Show box';
  await expect.soft(cspec, `${link}: from another page`).toHaveAttribute('open');
  await expect.soft(spec, `${link}: … that one only`).not.toHaveAttribute('open');
  await pg.locator('main.about nav.toc a[href="#model"]').click();
  await expect.soft(spec, `${link}: from the contents`).toHaveAttribute('open');
  await nav.getByRole('link', { name: 'Loop planner', exact: true }).click();
  await expect.soft(pg, '/about links back to the loop planner').toHaveURL(/\/loop$/);
  await pg.context().close();
}
