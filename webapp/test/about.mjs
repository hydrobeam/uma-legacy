import { PAGE_URL, check, artless, open, until, AUDIT } from './helpers.mjs';

export default async function about(b) {
  // "How this works": a link in the header to its own page (/about), which links back
  const pg = await open(b);
  // the header is the same on every page: the three pages in one order, the page you are on marked
  const tabs = () =>
    pg.evaluate(() =>
      [...document.querySelectorAll('header.top .ptabs a')]
        .map((a) => a.textContent + (a.getAttribute('aria-current') === 'page' ? '*' : ''))
        .join(','),
    );
  const t0 = await tabs();
  await pg.locator('header.top .tnav a', { hasText: 'How this works' }).click();
  await pg.waitForLoadState();
  check(
    'header: the same page tabs everywhere, the current page marked',
    t0 === 'Loop planner*,Career planner,How this works' &&
      (await tabs()) === 'Loop planner,Career planner,How this works*',
    `${t0} | ${await tabs()}`,
  );
  check(
    'header link opens /about',
    new URL(pg.url()).pathname === '/about' && /How this works/i.test(await pg.locator('main.about h2').innerText()),
  );
  const spec = pg.locator('main.about details.spec').first(),
    cspec = pg.locator('main.about details.spec').nth(1);
  check(
    '/about: the detailed model starts folded and opens to the model text; the version shows',
    !((await spec.getAttribute('open')) !== null) &&
      !/Model\./.test(await pg.locator('main.about').innerText()) &&
      (await (async () => {
        await spec.locator('summary').click();
        return /Model\./.test(await spec.innerText());
      })()) &&
      /Version v?\S+/.test(await pg.locator('main.about').innerText()),
  );
  check(
    '/about: the career planner in detail starts folded and opens to its steps',
    (await cspec.getAttribute('open')) === null &&
      (await (async () => {
        await cspec.locator('summary').click();
        const t = await cspec.innerText();
        return (
          /C1\. Legal careers/.test(t) &&
          /C8b\. Which careers are searched exactly/.test(t) &&
          /The Comparison panel/.test(t)
        );
      })()),
  );
  if (b.browserType().name() === 'chromium') {
    const f = artless(await pg.evaluate(AUDIT));
    check('/about: every text run meets WCAG AAA', !f.length, f.slice(0, 5).join(' | '));
  }
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
  const cOpen = (await cspec.getAttribute('open')) !== null,
    mShut = (await spec.getAttribute('open')) === null;
  await pg.locator('main.about nav.toc a[href="#model"]').click();
  check(
    '/about: a link to an "in detail" section opens its Show box',
    cOpen && mShut && (await until(pg, () => document.querySelector('main.about details.spec').open)),
  );
  await pg.locator('header.top .tnav a', { hasText: 'Loop planner' }).click();
  await pg.waitForSelector('#targets');
  check('/about links back to the planner', new URL(pg.url()).pathname === '/');
  await pg.context().close();
}
